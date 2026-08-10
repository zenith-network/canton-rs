use crate::{Choice, Name, Template, TemplateWithKey};

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

impl Template for TestTemplate {
    // type Choices = Coprod!(TestChoiceA, TestChoiceB);
}

pub struct TestKey;

impl TemplateWithKey for TestTemplate {
    type Key = TestKey;
}
