use crate::v2::{
    builder::{
        Binder, ChoiceBuilder, DataTypeBuilder, Field, InterfaceBuilder, InterfaceMethod, Kind,
        ModuleBuilder, PackageBuilder, TemplateBuilder, Type, TypeConRef, TypeParameter,
    },
    sealed,
};
use canton_types::PackageId;

#[test]
fn builds_package() {
    let dependency_id = PackageId::new("dependency".to_owned()).unwrap();

    let box_type = DataTypeBuilder::record("Box", [Field::new("value", Type::var("a"))])
        .type_parameter(TypeParameter::new("a", Kind::Star));
    let view_type = DataTypeBuilder::record("View", [Field::new("label", Type::text())]);
    let transfer_args = DataTypeBuilder::record("TransferArgs", []);
    let action_type = DataTypeBuilder::variant(
        "Action",
        [
            Field::new("Transfer", Type::local("Main", "TransferArgs")),
            Field::new("Noop", Type::unit()),
        ],
    );
    let status_type = DataTypeBuilder::enumeration("Status", ["Open", "Closed"]);
    let asset_data_type = DataTypeBuilder::interface("Asset");
    let contract_type = DataTypeBuilder::record(
        "Contract",
        [
            Field::new("owners", Type::list(Type::party())),
            Field::new("aux", Type::local("Aux", "AuxType")),
            Field::new(
                "boxedText",
                Type::tapp(Type::local("Main", "Box"), Type::text()),
            ),
            Field::new("payload", Type::imported(&dependency_id, "Dep", "Payload")),
        ],
    );
    let contract_template = TemplateBuilder::new("Contract")
        .choice(
            ChoiceBuilder::new(
                "Transfer",
                Type::local("Main", "TransferArgs"),
                Type::unit(),
            )
            .argument(Binder::new(
                "transferArgs",
                Type::local("Main", "TransferArgs"),
            )),
        )
        .key(Type::text())
        .implements(TypeConRef::local("Main", "Asset"));
    let asset_interface = InterfaceBuilder::new("Asset", Type::local("Main", "View"))
        .method(InterfaceMethod::new("view", Type::local("Main", "View")))
        .choice(ChoiceBuilder::new(
            "Inspect",
            Type::unit(),
            Type::local("Main", "View"),
        ));

    let main_module = ModuleBuilder::new("Main")
        .data_type(box_type)
        .data_type(view_type)
        .data_type(transfer_args)
        .data_type(action_type)
        .data_type(status_type)
        .data_type(asset_data_type)
        .data_type(contract_type)
        .template(contract_template)
        .interface(asset_interface);
    let aux_module = ModuleBuilder::new("Aux").data_type(DataTypeBuilder::record("AuxType", []));

    let package = PackageBuilder::new("main-package", "1.0.0")
        .module(main_module)
        .module(aux_module)
        .build();
    let sealed = sealed::Package::seal(&package).unwrap();
    let modules = sealed.modules();

    assert_eq!(modules.len(), 2);

    let main_module = modules[0];
    assert_eq!(
        main_module.name().iter().copied().collect::<Vec<_>>(),
        ["Main"]
    );
    assert_eq!(main_module.data_types().len(), 7);
    assert_eq!(main_module.templates().len(), 1);
    assert_eq!(main_module.interfaces().len(), 1);

    let template = main_module.templates()[0];
    assert_eq!(template.choices()[0].name(), "Transfer");
    assert_eq!(template.implements().len(), 1);

    let sealed::Type::Builtin(key_type) = template.key().unwrap().type_() else {
        panic!("template key should be a builtin type");
    };
    assert_eq!(key_type.type_(), sealed::BuiltinType::Text);

    let interface = main_module.interfaces()[0];
    assert_eq!(interface.methods()[0].name(), "view");
    assert_eq!(interface.choices()[0].name(), "Inspect");

    let contract = main_module.data_types()[6];
    let sealed::def_data_type::DataCons::Record(fields) = contract.data_cons() else {
        panic!("Contract should be a record");
    };
    let sealed::Type::Con(payload) = fields.fields()[3].type_() else {
        panic!("payload should be a type constructor");
    };
    let sealed::SelfOrImportedPackageId::ImportedPackageId(imported_package_id) =
        payload.tycon().module().package_id()
    else {
        panic!("payload should refer to an imported package");
    };

    assert_eq!(imported_package_id, dependency_id.as_str());
}
