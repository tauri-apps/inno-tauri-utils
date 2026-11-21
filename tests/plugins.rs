// Copyright 2026 inno-tauri-utils contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(all(windows, any(target_arch = "x86", target_arch = "x86_64")))]

//! `cargo test -p inno-integration-tests --target i686-pc-windows-msvc -- --ignored --nocapture`
//! Find ISCC.exe on PATH or set INNODIR. x64 installer tests require Inno Setup 7+.

use std::{
    env, fs, io,
    os::windows::process::CommandExt,
    path::Path,
    process::{Child, Command, ExitStatus},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const CREATE_NO_WINDOW: u32 = 0x08000000;

struct ChildGuard(Child);

impl ChildGuard {
    fn wait(&mut self) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.0.try_wait()? {
                return Ok(status);
            }
            assert!(
                Instant::now() < deadline,
                "child did not exit within 30 seconds"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn assert_pe_architecture(path: &Path, expected: u16) -> io::Result<()> {
    let bytes = fs::read(path)?;
    assert_eq!(&bytes[..2], b"MZ", "{} is not a PE file", path.display());
    let pe_offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&bytes[pe_offset..pe_offset + 4], b"PE\0\0");
    let machine = u16::from_le_bytes(bytes[pe_offset + 4..pe_offset + 6].try_into().unwrap());
    assert_eq!(machine, expected, "wrong architecture: {}", path.display());
    Ok(())
}

#[test]
#[ignore = "requires Inno Setup on PATH or in INNODIR"]
fn native_installer() -> io::Result<()> {
    let (architecture, target, machine) = if cfg!(target_arch = "x86_64") {
        ("x64", "x86_64-pc-windows-msvc", 0x8664)
    } else {
        ("x86", "i686-pc-windows-msvc", 0x014c)
    };
    let build_directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("inno-tests")
        .join(architecture);
    let run_id = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let run_directory = build_directory.join(run_id);
    fs::create_dir_all(&run_directory)?;

    let status = Command::new(env!("CARGO"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "build",
            "--locked",
            "--release",
            "--package",
            "inno-semvercompare",
            "--target",
            target,
        ])
        .arg("--target-dir")
        .arg(build_directory.join("build"))
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    assert!(status.success(), "DLL build failed: {status}");
    let release_directory = build_directory.join("build").join(target).join("release");
    assert_pe_architecture(&release_directory.join("inno_semvercompare.dll"), machine)?;

    let compiler = env::var_os("INNODIR")
        .filter(|directory| !directory.is_empty())
        .map(|directory| Path::new(&directory).join("ISCC.exe"))
        .unwrap_or_else(|| "ISCC.exe".into());
    let mut compiler = ChildGuard(
        Command::new(compiler)
            .arg("/Qp")
            .arg(format!("/DInstallerArchitecture={architecture}"))
            .arg(format!("/DDllDir={}", release_directory.display()))
            .arg(format!("/O{}", run_directory.display()))
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("smoke.iss"))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "Could not run Inno Setup: {error}. Add ISCC.exe to PATH or set INNODIR."
                    ),
                )
            })?,
    );
    let status = compiler.wait()?;
    assert!(status.success(), "Inno compilation failed: {status}");
    let installer = run_directory.join("smoke.exe");
    assert_pe_architecture(&installer, machine)?;

    let report = run_directory.join("results.txt");
    let log = run_directory.join("setup.log");
    let mut process = ChildGuard(
        Command::new(&installer)
            .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .arg(format!("/RESULTFILE={}", report.display()))
            .arg(format!("/LOG={}", log.display()))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?,
    );
    let status = process.wait()?;
    let results = fs::read_to_string(&report).unwrap_or_default();
    println!("{results}");
    assert!(
        status.success(),
        "installer failed: {status}; log: {}",
        log.display()
    );
    assert_eq!(
        results.lines().last(),
        Some("PASS"),
        "missing success report: {}",
        report.display()
    );
    println!(
        "Native {architecture} installer checks passed: {}",
        installer.display()
    );
    Ok(())
}
