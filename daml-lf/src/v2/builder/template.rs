use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::builder::{
    Type, TypeConRef,
    lower::{Lower, Lowerer},
};

/// An owned named value binder with its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binder {
    name: String,
    type_: Type,
}

impl Binder {
    pub fn new(name: impl Into<String>, type_: Type) -> Self {
        Self {
            name: name.into(),
            type_,
        }
    }
}

impl Lower<proto::VarWithType> for Binder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::VarWithType {
        proto::VarWithType {
            var_interned_str: lowerer.intern_string(self.name),
            r#type: Some(self.type_.lower(lowerer)),
        }
    }
}

/// Builder for a template choice. Expression fields are intentionally omitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceBuilder {
    name: String,
    consuming: bool,
    argument: Binder,
    return_type: Type,
    self_binder: String,
}

impl ChoiceBuilder {
    pub fn new(name: impl Into<String>, argument_type: Type, return_type: Type) -> Self {
        Self {
            name: name.into(),
            consuming: false,
            argument: Binder::new("arg", argument_type),
            return_type,
            self_binder: "self".to_owned(),
        }
    }

    pub fn consuming(mut self, consuming: bool) -> Self {
        self.consuming = consuming;
        self
    }

    pub fn argument(mut self, binder: Binder) -> Self {
        self.argument = binder;
        self
    }

    pub fn self_binder(mut self, name: impl Into<String>) -> Self {
        self.self_binder = name.into();
        self
    }
}

impl Lower<proto::TemplateChoice> for ChoiceBuilder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::TemplateChoice {
        proto::TemplateChoice {
            location: None,
            name_interned_str: lowerer.intern_string(self.name),
            consuming: self.consuming,
            controllers: None,
            observers: None,
            arg_binder: Some(self.argument.lower(lowerer)),
            ret_type: Some(self.return_type.lower(lowerer)),
            update: None,
            self_binder_interned_str: lowerer.intern_string(self.self_binder),
            authorizers: None,
        }
    }
}

/// Builder for a contract template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateBuilder {
    name: String,
    param: String,
    choices: Vec<ChoiceBuilder>,
    key: Option<Type>,
    implements: Vec<TypeConRef>,
}

impl TemplateBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param: "this".to_owned(),
            choices: Vec::new(),
            key: None,
            implements: Vec::new(),
        }
    }

    pub fn param(mut self, param: impl Into<String>) -> Self {
        self.param = param.into();
        self
    }

    pub fn choice(mut self, choice: ChoiceBuilder) -> Self {
        self.choices.push(choice);
        self
    }

    pub fn key(mut self, type_: Type) -> Self {
        self.key = Some(type_);
        self
    }

    pub fn implements(mut self, interface: TypeConRef) -> Self {
        self.implements.push(interface);
        self
    }
}

impl Lower<proto::DefTemplate> for TemplateBuilder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::DefTemplate {
        proto::DefTemplate {
            tycon_interned_dname: lowerer.intern_dotted(&self.name),
            param_interned_str: lowerer.intern_string(self.param),
            precond: None,
            signatories: None,
            choices: self
                .choices
                .into_iter()
                .map(|choice| choice.lower(lowerer))
                .collect(),
            observers: None,
            location: None,
            key: self.key.map(|type_| proto::def_template::DefKey {
                r#type: Some(type_.lower(lowerer)),
                key_expr: None,
                maintainers: None,
            }),
            implements: self
                .implements
                .into_iter()
                .map(|interface| proto::def_template::Implements {
                    interface: Some(interface.lower(lowerer)),
                    body: None,
                    location: None,
                })
                .collect(),
        }
    }
}

/// An owned method exposed by an interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceMethod {
    name: String,
    type_: Type,
}

impl InterfaceMethod {
    pub fn new(name: impl Into<String>, type_: Type) -> Self {
        Self {
            name: name.into(),
            type_,
        }
    }
}

impl Lower<proto::InterfaceMethod> for InterfaceMethod {
    fn lower(self, lowerer: &mut Lowerer) -> proto::InterfaceMethod {
        proto::InterfaceMethod {
            location: None,
            method_interned_name: lowerer.intern_string(self.name),
            r#type: Some(self.type_.lower(lowerer)),
        }
    }
}

/// Builder for interface metadata exposed by the sealed API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceBuilder {
    name: String,
    param: String,
    methods: Vec<InterfaceMethod>,
    choices: Vec<ChoiceBuilder>,
    view: Type,
    requires: Vec<TypeConRef>,
}

impl InterfaceBuilder {
    pub fn new(name: impl Into<String>, view: Type) -> Self {
        Self {
            name: name.into(),
            param: "this".to_owned(),
            methods: Vec::new(),
            choices: Vec::new(),
            view,
            requires: Vec::new(),
        }
    }

    pub fn param(mut self, param: impl Into<String>) -> Self {
        self.param = param.into();
        self
    }

    pub fn method(mut self, method: InterfaceMethod) -> Self {
        self.methods.push(method);
        self
    }

    pub fn choice(mut self, choice: ChoiceBuilder) -> Self {
        self.choices.push(choice);
        self
    }

    pub fn requires(mut self, interface: TypeConRef) -> Self {
        self.requires.push(interface);
        self
    }
}

impl Lower<proto::DefInterface> for InterfaceBuilder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::DefInterface {
        proto::DefInterface {
            location: None,
            tycon_interned_dname: lowerer.intern_dotted(&self.name),
            methods: self
                .methods
                .into_iter()
                .map(|method| method.lower(lowerer))
                .collect(),
            param_interned_str: lowerer.intern_string(self.param),
            choices: self
                .choices
                .into_iter()
                .map(|choice| choice.lower(lowerer))
                .collect(),
            view: Some(self.view.lower(lowerer)),
            requires: self
                .requires
                .into_iter()
                .map(|interface| interface.lower(lowerer))
                .collect(),
        }
    }
}
