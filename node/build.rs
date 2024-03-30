use std::{
    env, fs,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use miden_assembly::{
    ast::{AstSerdeOptions, ProgramAst},
    LibraryNamespace, MaslLibrary, Version,
};

const ASSETS_DIR: &str = "assets";
const ASM_DIR: &str = "asm";
const ASM_MIDEN_DIR: &str = "miden";
const ASM_NOTE_SCRIPTS_DIR: &str = "note_scripts";
const ASM_KERNELS_DIR: &str = "kernels/transaction";


// just for reference
fn main() -> io::Result<()> {
    // // re-build when the MASM code changes
    // println!("cargo:rerun-if-changed=asm");

    // // Copies the MASM code to the build directory
    // let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // let build_dir = env::var("OUT_DIR").unwrap();
    // let src = Path::new(&crate_dir).join(ASM_DIR);
    // let dst = Path::new(&build_dir).to_path_buf();
    // copy_directory(src, &dst);

    // // set source directory to {OUT_DIR}/asm
    // let source_dir = dst.join(ASM_DIR);

    // // set target directory to {OUT_DIR}/assets
    // let target_dir = Path::new(&build_dir).join(ASSETS_DIR);

    // // compile miden library
    // compile_miden_lib(&source_dir, &target_dir)?;

    // // compile kernel and note scripts
    // // compile_kernels(&source_dir.join(ASM_KERNELS_DIR), &target_dir.join("kernels"))?;
    // // compile_note_scripts(
    // //     &source_dir.join(ASM_NOTE_SCRIPTS_DIR),
    // //     &target_dir.join(ASM_NOTE_SCRIPTS_DIR),
    // // )?;

    Ok(())
}

fn copy_directory<T: AsRef<Path>, R: AsRef<Path>>(src: T, dst: R) {
    let mut prefix = src.as_ref().canonicalize().unwrap();
    // keep all the files inside the `asm` folder
    prefix.pop();

    let target_dir = dst.as_ref().join(ASM_DIR);
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).unwrap();
    }

    let dst = dst.as_ref();
    let mut todo = vec![src.as_ref().to_path_buf()];

    while let Some(goal) = todo.pop() {
        for entry in fs::read_dir(goal).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                let src_dir = path.canonicalize().unwrap();
                let dst_dir = dst.join(src_dir.strip_prefix(&prefix).unwrap());
                if !dst_dir.exists() {
                    fs::create_dir_all(&dst_dir).unwrap();
                }
                todo.push(src_dir);
            } else {
                let dst_file = dst.join(path.strip_prefix(&prefix).unwrap());
                fs::copy(&path, dst_file).unwrap();
            }
        }
    }
}

fn compile_miden_lib(source_dir: &Path, target_dir: &Path) -> io::Result<()> {
    let source_dir = source_dir.join(ASM_MIDEN_DIR);

    // if this build has the testing flag set, modify the code and reduce the cost of proof-of-work
    match env::var("CARGO_FEATURE_TESTING") {
        Ok(ref s) if s == "1" => {
            let constants = source_dir.join("kernels/tx/constants.masm");
            let patched = source_dir.join("kernels/tx/constants.masm.patched");

            // scope for file handlers
            {
                let read = File::open(&constants).unwrap();
                let mut write = File::create(&patched).unwrap();
                let modified = BufReader::new(read).lines().map(decrease_pow);

                for line in modified {
                    write.write_all(line.unwrap().as_bytes()).unwrap();
                    write.write_all(&[b'\n']).unwrap();
                }
                write.flush().unwrap();
            }

            fs::remove_file(&constants).unwrap();
            fs::rename(&patched, &constants).unwrap();
        },
        _ => (),
    }

    let ns = LibraryNamespace::try_from("miden".to_string()).expect("invalid base namespace");
    let version = Version::try_from(env!("CARGO_PKG_VERSION")).expect("invalid cargo version");
    let miden_lib = MaslLibrary::read_from_dir(source_dir, ns, true, version)?;

    miden_lib.write_to_dir(target_dir)?;

    Ok(())
}

fn decrease_pow(line: io::Result<String>) -> io::Result<String> {
    let mut line = line?;
    if line.starts_with("const.REGULAR_ACCOUNT_SEED_DIGEST_MODULUS") {
        line.clear();
        // 2**5
        line.push_str("const.REGULAR_ACCOUNT_SEED_DIGEST_MODULUS=32 # reduced via build.rs");
    } else if line.starts_with("const.FAUCET_ACCOUNT_SEED_DIGEST_MODULUS") {
        line.clear();
        // 2**6
        line.push_str("const.FAUCET_ACCOUNT_SEED_DIGEST_MODULUS=64 # reduced via build.rs");
    }
    Ok(line)
}
