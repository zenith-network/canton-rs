//! Decodes the compiler-produced stable Daml-LF 2.4 fixture
//! `codegen/tests/assets/stable-lf24.dar` (SDK 3.6.0: damlc 3.6.2, Canton 3.6.1;
//! built from `codegen/tests/assets/stable-lf24` with `dpm build -o ../stable-lf24.dar`).
#![cfg(feature = "v2")]

use daml_lf::dar::DarFile;
use daml_lf::package::VersionedPackage;
use daml_lf::proto::com::digitalasset::daml::lf::archive::v2::{BuiltinFunction, Expr, expr};
use daml_lf_version::{MinorVersion, Version};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/codegen/tests/assets/stable-lf24.dar"
);

const LF_2_4: Version = Version {
    major: 2,
    minor: MinorVersion::Stable { version: 4 },
};

/// Collects the raw builtin ids reachable through the expression forms that wrap
/// builtin applications (applications, abstractions and let blocks).
fn collect_builtins(expr: &Expr, out: &mut Vec<i32>) {
    let Some(sum) = &expr.sum else { return };
    match sum {
        expr::Sum::Builtin(id) => out.push(*id),
        expr::Sum::App(app) => {
            app.fun.iter().for_each(|e| collect_builtins(e, out));
            app.args.iter().for_each(|e| collect_builtins(e, out));
        }
        expr::Sum::TyApp(ty_app) => ty_app.expr.iter().for_each(|e| collect_builtins(e, out)),
        expr::Sum::Abs(abs) => abs.body.iter().for_each(|e| collect_builtins(e, out)),
        expr::Sum::TyAbs(ty_abs) => ty_abs.body.iter().for_each(|e| collect_builtins(e, out)),
        expr::Sum::Let(block) => {
            for binding in &block.bindings {
                binding.bound.iter().for_each(|e| collect_builtins(e, out));
            }
            block.body.iter().for_each(|e| collect_builtins(e, out));
        }
        _ => {}
    }
}

#[test]
fn decodes_every_package_of_the_stable_lf24_dar() {
    let mut dar = DarFile::read_from(FIXTURE).expect("fixture DAR");
    let main = dar.main_dalf().expect("main DALF");
    assert_eq!(*main.version(), LF_2_4);
    let dalfs = dar.dalfs().expect("DALFs");
    assert!(dalfs.iter().any(|dalf| *dalf.version() == LF_2_4));
    for dalf in dalfs {
        let package = dalf.to_package().expect("supported, decodable package");
        package.seal().expect("well-formed package");
    }
}

/// daml-stdlib 3.6.2 at LF 2.4 implements `DA.ExternalCall.externalCall` with the
/// stable `EXTERNAL_CALL` builtin (74); every builtin of the fixture must map to a
/// known `BuiltinFunction`.
#[test]
fn recognizes_the_stable_external_call_builtin() {
    let mut dar = DarFile::read_from(FIXTURE).expect("fixture DAR");
    let mut builtins = Vec::new();
    for dalf in dar.dalfs().expect("DALFs") {
        let package = dalf.to_package().expect("package");
        let VersionedPackage::V2(package) = package.versioned();
        for module in &package.modules {
            for value in &module.values {
                value
                    .expr
                    .iter()
                    .for_each(|e| collect_builtins(e, &mut builtins));
            }
        }
        for e in &package.interned_exprs {
            collect_builtins(e, &mut builtins);
        }
    }
    assert!(builtins.contains(&74), "no builtin 74 in the fixture");
    for id in builtins {
        BuiltinFunction::try_from(id).unwrap_or_else(|_| panic!("unknown builtin {id}"));
    }
    assert_eq!(
        BuiltinFunction::try_from(74),
        Ok(BuiltinFunction::ExternalCall)
    );
}
