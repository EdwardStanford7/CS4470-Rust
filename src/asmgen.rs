use crate::ast::*;
use crate::typechecker::TypeEnvironment;
use std::{
    cell::RefCell,
    rc::Rc,
};




pub fn asmgen(
    mut commands: Vec<Command<'_>>,
    env: Rc<RefCell<TypeEnvironment<'_>>>
) -> Rc<str> {
    let owned: Rc<str> = Rc::from(String::from("hi"));
    return owned;
}
