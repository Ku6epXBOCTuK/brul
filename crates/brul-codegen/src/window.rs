use brul_syntax::ast::{Expr, ExprKind, Position, UnaryOp};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Declaration, Element};
use brul_window::{WindowConfig, WindowPosition};

use crate::error_at;

pub struct HandlerArg {
    pub event: String,
    pub function: String,
    pub position: Position,
}

pub struct AttachParam {
    pub name: String,
    pub value: Expr,
}

pub struct AttachArg {
    pub name: String,
    pub params: Vec<AttachParam>,
    pub position: Position,
}

pub struct WindowArgs {
    pub config: WindowConfig,
    pub handlers: Vec<HandlerArg>,
    pub attaches: Vec<AttachArg>,
}

pub fn arguments(declaration: &Declaration) -> Result<WindowArgs, Error> {
    let mut config = WindowConfig {
        frameless: false,
        transparent: false,
        ..WindowConfig::default()
    };
    let mut handlers = Vec::new();
    let mut attaches = Vec::new();
    let mut x = None;
    let mut y = None;
    for arg in declaration.args() {
        match arg {
            Arg::Named { name, value } => match name.as_str() {
                "x" => x = Some(coordinate(value, "x")?),
                "y" => y = Some(coordinate(value, "y")?),
                _ => apply(&mut config, name, value)?,
            },
            Arg::Handler { event, handler } => handlers.push(HandlerArg {
                event: event.clone(),
                function: handler.clone(),
                position: declaration.position,
            }),
            Arg::Attach {
                name,
                params,
                position,
            } => attaches.push(AttachArg {
                name: name.clone(),
                params: params
                    .iter()
                    .map(|(name, value)| AttachParam {
                        name: name.clone(),
                        value: value.clone(),
                    })
                    .collect(),
                position: *position,
            }),
            Arg::Class(_) | Arg::Anchor(_) | Arg::Name(_) => {
                return Err(error_at(
                    declaration.position,
                    "window arguments must be named",
                ));
            }
        }
    }
    config.position = match (x, y) {
        (Some(x), Some(y)) => Some(WindowPosition { x, y }),
        (None, None) => None,
        _ => {
            return Err(error_at(
                declaration.position,
                "window position needs both 'x' and 'y'",
            ));
        }
    };
    if config.centered && config.position.is_some() {
        return Err(error_at(
            declaration.position,
            "'centered' conflicts with 'x' and 'y'",
        ));
    }
    config.validate().map_err(|error| {
        error_at(
            declaration.position,
            format!("window arguments are invalid: {error}"),
        )
    })?;
    Ok(WindowArgs {
        config,
        handlers,
        attaches,
    })
}

fn coordinate(value: &Expr, name: &str) -> Result<f64, Error> {
    let number = match &value.kind {
        ExprKind::Number(number) => Some(*number),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match &operand.kind {
            ExprKind::Number(number) => Some(-number),
            _ => None,
        },
        _ => None,
    };
    match number {
        Some(number) if number.is_finite() => Ok(number),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a finite number"),
        )),
    }
}

fn apply(config: &mut WindowConfig, name: &str, value: &Expr) -> Result<(), Error> {
    match name {
        "title" => config.title = string(value, "title")?,
        "width" => config.width = size(value, "width")?,
        "height" => config.height = size(value, "height")?,
        "frameless" => config.frameless = flag(value, "frameless")?,
        "transparent" => config.transparent = flag(value, "transparent")?,
        "visible" => config.visible = flag(value, "visible")?,
        "centered" => config.centered = flag(value, "centered")?,
        "scroll" => config.scroll = flag(value, "scroll")?,
        "icon" => {
            if !matches!(value.kind, ExprKind::Ident(_)) {
                return Err(error_at(
                    value.position,
                    "window argument 'icon' must be a resource name",
                ));
            }
        }
        _ => {
            return Err(error_at(
                value.position,
                format!("unknown window argument '{name}'"),
            ));
        }
    }
    Ok(())
}

pub fn root(declaration: &Declaration) -> Result<&Element, Error> {
    crate::root::window_root(declaration)
}

fn string(value: &Expr, name: &str) -> Result<String, Error> {
    match &value.kind {
        ExprKind::String(text) => Ok(text.clone()),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a string"),
        )),
    }
}

fn size(value: &Expr, name: &str) -> Result<u32, Error> {
    match &value.kind {
        ExprKind::Number(number)
            if number.is_finite()
                && *number > 0.0
                && number.fract() == 0.0
                && *number <= f64::from(u32::MAX) =>
        {
            Ok(*number as u32)
        }
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a positive integer"),
        )),
    }
}

fn flag(value: &Expr, name: &str) -> Result<bool, Error> {
    match &value.kind {
        ExprKind::Bool(flag) => Ok(*flag),
        _ => Err(error_at(
            value.position,
            format!("window argument '{name}' must be a bool"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use brul_syntax::markup::DeclarationKind;

    use super::*;

    fn window(args: Vec<Arg>) -> Declaration {
        Declaration {
            name: "Demo".to_owned(),
            members: Vec::new(),
            position: Position::default(),
            kind: DeclarationKind::Window { args },
        }
    }

    fn flag_arg(name: &str, value: bool) -> Arg {
        Arg::Named {
            name: name.to_owned(),
            value: Expr::new(ExprKind::Bool(value), Position::default()),
        }
    }

    fn number_arg(name: &str, value: f64) -> Arg {
        Arg::Named {
            name: name.to_owned(),
            value: Expr::new(ExprKind::Number(value), Position::default()),
        }
    }

    #[test]
    fn a_window_can_be_placed_or_centered() {
        let args = vec![number_arg("x", -40.0), number_arg("y", 120.0)];
        let config = arguments(&window(args)).expect("arguments").config;
        assert_eq!(config.position, Some(WindowPosition { x: -40.0, y: 120.0 }));

        let config = arguments(&window(vec![flag_arg("centered", true)]))
            .expect("arguments")
            .config;
        assert!(config.centered);
        assert!(config.position.is_none());
    }

    #[test]
    fn a_window_accepts_an_attachment() {
        let args = vec![Arg::Attach {
            name: "keys".to_owned(),
            params: Vec::new(),
            position: Position::default(),
        }];
        let args = arguments(&window(args)).expect("arguments");
        assert_eq!(args.attaches.len(), 1);
        assert_eq!(args.attaches[0].name, "keys");
    }

    #[test]
    fn a_half_position_is_rejected() {
        let error = arguments(&window(vec![number_arg("x", 10.0)]))
            .err()
            .expect("error");
        assert_eq!(error.message(), "window position needs both 'x' and 'y'");
    }

    #[test]
    fn a_centered_window_cannot_carry_a_position() {
        let args = vec![
            flag_arg("centered", true),
            number_arg("x", 0.0),
            number_arg("y", 0.0),
        ];
        let error = arguments(&window(args)).err().expect("error");
        assert_eq!(error.message(), "'centered' conflicts with 'x' and 'y'");
    }

    #[test]
    fn a_plain_window_is_decorated_and_opaque() {
        let config = arguments(&window(Vec::new())).expect("arguments").config;

        assert!(!config.frameless);
        assert!(!config.transparent);
    }

    #[test]
    fn a_window_can_start_hidden() {
        let args = vec![flag_arg("visible", false)];
        let config = arguments(&window(args)).expect("arguments").config;

        assert!(!config.visible);
        assert!(WindowConfig::default().visible);
    }

    #[test]
    fn a_window_scrolls_its_root_unless_it_says_otherwise() {
        let config = arguments(&window(Vec::new())).expect("arguments").config;
        assert!(config.scroll);

        let args = vec![flag_arg("scroll", false)];
        let config = arguments(&window(args)).expect("arguments").config;
        assert!(!config.scroll);
    }

    #[test]
    fn explicit_false_flags_are_accepted() {
        let args = vec![flag_arg("frameless", false), flag_arg("transparent", false)];
        let config = arguments(&window(args)).expect("arguments").config;

        assert!(!config.frameless);
        assert!(!config.transparent);
    }

    #[test]
    fn apply_rejects_an_unknown_argument() {
        let mut config = WindowConfig::default();
        let value = Expr::new(ExprKind::Number(1.0), Position { line: 1, column: 1 });
        let error = apply(&mut config, "bogus", &value).unwrap_err();
        assert_eq!(error.message(), "unknown window argument 'bogus'");
        assert_eq!((error.line(), error.column()), (1, 1));
    }
}
