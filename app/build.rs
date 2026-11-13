//! Windows: 把 app.rc（图标 + 版本信息）编进 IslandHop.exe。
//!
//! 做法与岛仔一致：不引入 winres / winresource 之类的新依赖，直接找
//! Windows SDK 的 rc.exe，把生成的 .res 通过 rustc-link-arg 交给链接器。
//! 非 Windows 目标（含 macOS CI 与本机开发）直接返回，不影响构建。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn find_rc() -> Option<PathBuf> {
    let roots = [
        r"C:\Program Files (x86)\Windows Kits\10\bin",
        r"C:\Program Files\Windows Kits\10\bin",
    ];
    let mut found = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            let rc = entry.path().join("x64").join("rc.exe");
            if rc.exists() { found.push(rc); }
        }
    }
    found.sort();
    found.pop()
}

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=assets/AppIcon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("islandhop.res");
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe 未找到，跳过 Windows 图标资源");
        return;
    };
    // 显式指定工作目录与 include 路径，避免 CI runner 的 cwd 差异让
    // app.rc 里的 "assets/AppIcon.ico" 找不到。
    let ok = Command::new(&rc)
        .current_dir(&manifest)
        .args(["/nologo", "/fo"])
        .arg(&out)
        .arg("/i")
        .arg(&manifest)
        .arg("app.rc")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        println!("cargo:warning=rc.exe 编译 app.rc 失败，exe 将没有自定义图标");
        return;
    }
    println!("cargo:rustc-link-arg={}", out.display());
}
