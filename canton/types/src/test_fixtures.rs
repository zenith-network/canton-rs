use crate::{Choice, Implements, Interface, Name, Template, TemplateOrInterface, TemplateWithKey};

pub struct TestTemplate;

pub struct TestChoiceA;

impl Choice<TestTemplate> for TestChoiceA {
    const CONSUMING: bool = false;

    const NAME: Name = Name::new_static_unchecked("TestChoiceA");

    type Result = ();
}

pub struct TestChoiceB;

impl Choice<TestTemplate> for TestChoiceB {
    const CONSUMING: bool = true;

    const NAME: Name = Name::new_static_unchecked("TestChoiceA");

    type Result = ();
}

impl Template for TestTemplate {}

impl TemplateOrInterface for TestTemplate {}

pub struct TestKey;

impl TemplateWithKey for TestTemplate {
    type Key = TestKey;
}

pub struct TestViewType;

pub struct TestInterface;

impl Interface for TestInterface {
    type View = TestViewType;
}

impl TemplateOrInterface for TestInterface {}

impl Implements<TestInterface> for TestTemplate {}
