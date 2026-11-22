use std::{fs, path::Path};

use naga::{
    front::wgsl::parse_str,
    valid::{Capabilities, ValidationFlags, Validator},
};

pub const PRELUDE: &str = include_str!("effect_prelude.wgsl");
pub const EFFECT_FRAME_SIZE: u64 = size_of::<EffectFrame>() as u64;
pub const EFFECT_INSTANCE_SIZE: u64 = size_of::<EffectInstance>() as u64;

const FRAGMENT: &str = "
@fragment
fn effect_fs(in: EffectVaryings) -> @location(0) vec4<f32> {
    let input = effect_make_input(in, effect_quads[in.index]);
    let color = shade(input);
    return vec4<f32>(color.rgb * color.a, color.a);
}
";

const ADVANCE: &str = "
@fragment
fn effect_advance_fs(in: EffectVaryings) -> @location(0) vec4<f32> {
    let input = effect_make_input(in, effect_quads[in.index]);
    return advance(input);
}
";

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EffectFrame {
    pub time: f32,
    pub delta: f32,
    pub frame: u32,
    pub pad: u32,
    pub viewport: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EffectInstance {
    pub bounds: [f32; 4],
    pub uv: [f32; 4],
    pub params: [[f32; 4]; 16],
    pub meta: [f32; 4],
    pub flags: [u32; 4],
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EffectError {
    #[error("cannot read effect shader {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}:{line}:{column}: {message}")]
    Parse {
        path: String,
        line: u32,
        column: u32,
        message: String,
    },
    #[error("{path}: invalid effect shader: {message}")]
    Validate { path: String, message: String },
    #[error("{path}: missing effect entry point `shade`")]
    MissingShade { path: String },
}

#[derive(Clone, Debug)]
pub struct EffectShader {
    source: String,
    advance: bool,
}

impl EffectShader {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn has_advance(&self) -> bool {
        self.advance
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<EffectShader, EffectError> {
    let path = path.as_ref();
    let name = path.to_string_lossy().into_owned();
    let source = fs::read_to_string(path).map_err(|source| EffectError::Io {
        path: name.clone(),
        source,
    })?;
    compile(&source, &name)
}

pub fn compile(source: &str, name: &str) -> Result<EffectShader, EffectError> {
    let offset = PRELUDE.lines().count() as u32 + 1;
    let base = format!("{PRELUDE}\n{source}");
    let module = parse_checked(&base, name, offset)?;
    if !has_function(&module, "shade") {
        return Err(EffectError::MissingShade {
            path: name.to_owned(),
        });
    }
    let advance = has_function(&module, "advance");
    let tail = if advance {
        format!("{FRAGMENT}\n{ADVANCE}")
    } else {
        FRAGMENT.to_owned()
    };
    let full = format!("{base}\n{tail}");
    parse_checked(&full, name, offset)?;
    Ok(EffectShader {
        source: full,
        advance,
    })
}

pub fn effect_frame_bytes(frame: &EffectFrame) -> [u8; EFFECT_FRAME_SIZE as usize] {
    let mut out = [0u8; EFFECT_FRAME_SIZE as usize];
    out[0..4].copy_from_slice(&frame.time.to_ne_bytes());
    out[4..8].copy_from_slice(&frame.delta.to_ne_bytes());
    out[8..12].copy_from_slice(&frame.frame.to_ne_bytes());
    out[12..16].copy_from_slice(&frame.pad.to_ne_bytes());
    for (index, value) in frame.viewport.iter().enumerate() {
        let start = 16 + index * 4;
        out[start..start + 4].copy_from_slice(&value.to_ne_bytes());
    }
    out
}

pub fn effect_instance_bytes(instances: &[EffectInstance]) -> Vec<u8> {
    let mut out = Vec::with_capacity(instances.len() * EFFECT_INSTANCE_SIZE as usize);
    for instance in instances {
        for value in instance.bounds {
            out.extend_from_slice(&value.to_ne_bytes());
        }
        for value in instance.uv {
            out.extend_from_slice(&value.to_ne_bytes());
        }
        for row in instance.params {
            for value in row {
                out.extend_from_slice(&value.to_ne_bytes());
            }
        }
        for value in instance.meta {
            out.extend_from_slice(&value.to_ne_bytes());
        }
        for value in instance.flags {
            out.extend_from_slice(&value.to_ne_bytes());
        }
    }
    out
}

fn parse_checked(source: &str, name: &str, offset: u32) -> Result<naga::Module, EffectError> {
    let module = parse_str(source).map_err(|error| {
        let (line, column) = error
            .location(source)
            .map(|location| (location.line_number, location.line_position))
            .unwrap_or((0, 0));
        EffectError::Parse {
            path: name.to_owned(),
            line: line.saturating_sub(offset).max(1),
            column,
            message: error.message().to_owned(),
        }
    })?;
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|error| EffectError::Validate {
            path: name.to_owned(),
            message: error.to_string(),
        })?;
    Ok(module)
}

fn has_function(module: &naga::Module, name: &str) -> bool {
    module
        .functions
        .iter()
        .any(|(_, function)| function.name.as_deref() == Some(name))
}
