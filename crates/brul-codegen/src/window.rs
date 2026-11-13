use brul_syntax::ast::{Expr, ExprKind, Position, UnaryOp};
use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Element, Member, Window};
use brul_window::{WindowConfig, WindowPosition};

use crate::error_at;

pub struct HandlerArg {
    pub event: String,
    pub function: String,
    pub position: Position,
}

pub fn arguments(window: &Window) -> Result<(WindowConfig, Vec<HandlerArg>), Error> {
    let mut config = WindowConfig {
        frameless: false,
        transparent: false,
        ..WindowConfig::default()
    };
    let mut handlers = Vec::new();
    let mut x = None;
    let mut y = None;
    for arg in &window.args {
        match arg {
            Arg::Named { name, value } => match name.as_str() {
                "x" => x = Some(coordinate(value, "x")?),
                "y" => y = Some(coordinate(value, "y")?),
                _ => apply(&mut config, name, value)?,
            },
            Arg::Handler { event, handler } => handlers.push(HandlerArg {
                event: event.clone(),
                function: handler.clone(),
                position: window.position,
            }),
            Arg::Class(_) => {
                return Err(error_at(window.position, "window arguments must be named"));
            }
        }
    }
    config.position = match (x, y) {
        (Some(x), Some(y)) => Some(WindowPosition { x, y }),
        (None, None) => None,
        _ => {
            return Err(error_at(
                window.position,
                "window position needs both 'x' and 'y'",
            ));
        }
    };
    if config.centered && config.position.is_some() {
        return Err(error_at(
            window.position,
            "'centered' conflicts with 'x' and 'y'",
        ));
    }
    config.validate().map_err(|error| {
        error_at(
            window.position,
            format!("window arguments are invalid: {error}"),
        )
    })?;
    Ok((config, handlers))
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
        _ => {
            return Err(error_at(
                value.position,
                format!("unknown window argument '{name}'"),
            ));
        }
    }
    Ok(())
}

pub fn root(window: &Window) -> Result<&Element, Error> {
    let mut roots = window.members.iter().filter_map(|member| match member {
        Member::Element(element) => Some(element),
        Member::Prop(_) | Member::Event(_) => None,
    });
    let root = roots
        .next()
        .ok_or_else(|| error_at(window.position, "window has no root element"))?;
    if roots.next().is_some() {
        return Err(error_at(
            window.position,
            "window has more than one root element",
        ));
    }
    Ok(root)
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
    use super::*;

    fn window(args: Vec<Arg>) -> Window {
        Window {
            name: "Demo".to_owned(),
            args,
            members: Vec::new(),
            position: Position::default(),
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
        let (config, _) = arguments(&window(args)).expect("arguments");
        assert_eq!(config.position, Some(WindowPosition { x: -40.0, y: 120.0 }));

        let (config, _) = arguments(&window(vec![flag_arg("centered", true)])).expect("arguments");
        assert!(config.centered);
        assert!(config.position.is_none());
    }

    #[test]
    fn a_half_position_is_rejected() {
        let error = arguments(&window(vec![number_arg("x", 10.0)]))
            .err()
            .expect("error");
        assert_eq!(error.message, "window position needs both 'x' and 'y'");
    }

    #[test]
    fn a_centered_window_cannot_carry_a_position() {
        let args = vec![
            flag_arg("centered", true),
            number_arg("x", 0.0),
            number_arg("y", 0.0),
        ];
        let error = arguments(&window(args)).err().expect("error");
        assert_eq!(error.message, "'centered' conflicts with 'x' and 'y'");
    }

    #[test]
    fn a_plain_window_is_decorated_and_opaque() {
        let (config, _) = arguments(&window(Vec::new())).expect("arguments");

        assert!(!config.frameless);
        assert!(!config.transparent);
    }

    #[test]
    fn a_window_can_start_hidden() {
        let args = vec![flag_arg("visible", false)];
        let (config, _) = arguments(&window(args)).expect("arguments");

        assert!(!config.visible);
        assert!(WindowConfig::default().visible);
    }

    #[test]
    fn a_window_scrolls_its_root_unless_it_says_otherwise() {
        let (config, _) = arguments(&window(Vec::new())).expect("arguments");
        assert!(config.scroll);

        let args = vec![flag_arg("scroll", false)];
        let (config, _) = arguments(&window(args)).expect("arguments");
        assert!(!config.scroll);
    }

    #[test]
    fn explicit_false_flags_are_accepted() {
        let args = vec![flag_arg("frameless", false), flag_arg("transparent", false)];
        let (config, _) = arguments(&window(args)).expect("arguments");

        assert!(!config.frameless);
        assert!(!config.transparent);
    }

    #[test]
    fn apply_rejects_an_unknown_argument() {
        let mut config = WindowConfig::default();
        let value = Expr::new(ExprKind::Number(1.0), Position { line: 1, column: 1 });
        let error = apply(&mut config, "bogus", &value).unwrap_err();
        assert_eq!(error.message, "unknown window argument 'bogus'");
        assert_eq!((error.line, error.column), (1, 1));
    }
}
