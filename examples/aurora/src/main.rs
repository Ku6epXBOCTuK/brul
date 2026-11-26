use std::process::ExitCode;

use brul::Value;

mod generated;

brul::effect! {
    name: "Aurora",
    shader: "aurora.wgsl",
    animated: true,
    pub struct AuroraParams {
        phase: f32,
        tint: [f32; 4],
        glow: [f32; 4],
    }
}

fn main() -> ExitCode {
    brul::logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aurora: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let seeds = [("phase", Value::Number(0.0))];
    let backend = match std::env::var("BRUL_EXAMPLE_BACKEND") {
        Ok(value) => Some(value.parse::<brul::Backend>()?),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let mut windows = generated::windows();
    if let Some(backend) = backend {
        for window in &mut windows {
            window.config.backend = backend;
        }
    }
    brul::run_generated(
        windows,
        generated::script(),
        generated::names(),
        None,
        &seeds,
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use brul::effect::EffectParams;
    use brul::{BindingTarget, Component, Script, Value};

    use super::{AuroraParams, BRUL_EFFECT_AURORA, generated};

    fn component() -> brul::ComponentHandle {
        let component: brul::ComponentHandle = Rc::new(RefCell::new(Component::new()));
        let mut state = component.borrow_mut();
        state.set_names(generated::names());
        state
            .declare("phase", Value::Number(0.25), false)
            .expect("seed");
        match generated::aurora_script().expect("script") {
            Script::Typed { init, .. } => init(&mut state).expect("script init"),
            _ => panic!("unexpected script kind"),
        }
        drop(state);
        component
    }

    fn built(component: &brul::ComponentHandle) -> brul::Built {
        let mut build = brul::Build::new(brul::Services::default());
        generated::aurora_view(Some(generated::ThemeId::Dark), &mut build, component)
            .expect("generated view")
    }

    #[test]
    fn the_shader_compiles() {
        BRUL_EFFECT_AURORA.compile().expect("valid WGSL");
        const { assert!(BRUL_EFFECT_AURORA.animated) };
    }

    #[test]
    fn style_arguments_reach_the_effect_uniforms() {
        let component = component();
        let built = built(&component);
        let (def, args) = built
            .bindings
            .iter()
            .find_map(|binding| match &binding.target {
                BindingTarget::Effect { def, args } => Some((def, args)),
                _ => None,
            })
            .expect("effect binding");
        assert_eq!(def.name, "Aurora");
        assert!(def.animated);
        let values = {
            let mut state = component.borrow_mut();
            args.iter()
                .map(|arg| arg(&mut state).expect("effect argument"))
                .collect::<Vec<_>>()
        };
        let actual = def.pack(&values).expect("packed uniforms");
        let expected = AuroraParams {
            phase: 0.25,
            tint: [0.25, 0.85, 0.65, 0.90],
            glow: [9.0, 0.35, 0.22, 0.45],
        }
        .pack();
        assert_eq!(actual, expected);
    }

    #[test]
    fn the_grid_carries_the_effect_to_the_snapshot() {
        let component = component();
        let built = built(&component);
        let mut tree = brul::gui::WidgetTree::new(built.widget);
        let mut engine = brul::gui::TextEngine::new();
        tree.layout(
            &mut engine,
            brul::gui::Size {
                width: 640,
                height: 360,
            },
            1.0,
        )
        .expect("layout");
        let snapshot = tree.snapshot().expect("snapshot");
        let panes = snapshot
            .nodes
            .iter()
            .filter(|node| {
                node.effect
                    .as_ref()
                    .is_some_and(|spec| spec.name == "Aurora")
            })
            .collect::<Vec<_>>();
        assert_eq!(panes.len(), 1, "the styled pane lost its effect");
        let pane = panes[0];
        assert!(pane.width > 0 && pane.height > 0, "the pane has no area");
        assert!(
            snapshot
                .nodes
                .iter()
                .any(|node| node.x > pane.x && node.y == pane.y),
            "the grid has no second column"
        );
        assert!(
            snapshot.nodes.iter().any(|node| node.y > pane.y),
            "the grid has no second row"
        );
    }
}
