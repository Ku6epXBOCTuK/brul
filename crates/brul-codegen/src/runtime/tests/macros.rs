macro_rules! typed_expr {
    ($macro:ident, $name:literal, $ty:ty) => {{
        fn read(component: &mut Component) -> Result<$ty, ScriptError> {
            (&component.read($name)?).try_into()
        }
        read
    }};
}

macro_rules! expr {
    ($name:literal) => {{
        fn read(component: &mut Component) -> Result<Dyn, ScriptError> {
            Ok(Dyn::from(component.read($name)?))
        }
        read
    }};
}

macro_rules! text {
    ($name:literal) => {
        typed_expr!(text, $name, String)
    };
}

macro_rules! flag {
    ($name:literal) => {
        typed_expr!(flag, $name, bool)
    };
}

macro_rules! number {
    ($name:literal) => {
        typed_expr!(number, $name, f64)
    };
}
