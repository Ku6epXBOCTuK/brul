use std::collections::HashSet;
use std::fs;
use std::path::Path;

use brul_style_schema::{
    FIELDS, FieldDefault, FieldSpec, FieldType, LayoutField, PROPERTIES, PatchField, PatchTarget,
    PropertyKey, PropertySpec, SHORTHANDS, ShorthandParser, Side, StatePolicy, StyleField,
    StyleOwner, Target, TweenKind, Type,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub enum Output {
    Style,
    Syntax,
    Layout,
    Widget,
    Text,
    Codegen,
    CodegenFields,
}

pub fn generate(output: Output, out_dir: &Path) {
    let (file, tokens) = match output {
        Output::Style => ("style_properties.rs", style()),
        Output::Syntax => ("style_shorthands.rs", syntax()),
        Output::Layout => ("style_layout.rs", layout_style()),
        Output::Widget => ("style_widget.rs", widget()),
        Output::Text => ("style_text.rs", text()),
        Output::Codegen => ("style_kind.rs", codegen()),
        Output::CodegenFields => ("style_fields.rs", codegen_fields()),
    };
    let parsed = syn::parse2::<syn::File>(tokens).expect("generated style code parses");
    fs::write(out_dir.join(file), prettyplease::unparse(&parsed))
        .expect("cannot write generated style code");
}

fn style() -> TokenStream {
    let mut variants = Vec::new();
    let mut seen = HashSet::new();
    for spec in PROPERTIES {
        let (name, sided) = variant(spec.key);
        if seen.insert(name.to_string()) {
            variants.push(if sided {
                quote!(#name(Side))
            } else {
                quote!(#name)
            });
        }
    }
    let specs = PROPERTIES.iter().map(spec);
    let kinds = PROPERTIES.iter().map(|spec| kind(spec.key));
    quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Kind {
            #(#variants),*
        }

        pub static SPECS: &[Spec] = &[#(#specs),*];

        impl Kind {
            pub const ALL: &'static [Kind] = &[#(#kinds),*];

            pub fn spec(self) -> &'static Spec {
                SPECS.iter().find(|spec| spec.kind == self).expect("SPECS lists every kind")
            }

            pub fn name(self) -> &'static str {
                self.spec().name
            }

            pub fn from_name(name: &str) -> Option<Kind> {
                SPECS.iter().find(|spec| spec.name == name).map(|spec| spec.kind)
            }

            pub fn theme_type(self) -> Option<ThemeType> {
                self.spec().theme_type()
            }

            pub fn in_state(self) -> StatePolicy {
                self.spec().in_state
            }

            pub fn target(self) -> Target {
                self.spec().target
            }

            pub fn animatable(self) -> bool {
                self.spec().animatable
            }
        }
    }
}

fn syntax() -> TokenStream {
    let shorthands = SHORTHANDS.iter().map(|spec| {
        let name = spec.name;
        let kinds = spec.expands_to.iter().map(|key| kind(*key));
        let parser = parser(spec.parser);
        quote!(Shorthand { name: #name, kinds: &[#(#kinds),*], parse: #parser })
    });
    quote! {
        pub static SHORTHANDS: &[Shorthand] = &[#(#shorthands),*];
    }
}

fn layout_style() -> TokenStream {
    let specs = fields(StyleOwner::Layout);
    let style_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: #ty,)
        })
        .collect();
    let patch_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: Option<#ty>,)
        })
        .collect();
    let defaults: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            quote!(#name: None,)
        })
        .collect();
    let diff: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            if is_copy(spec.ty) {
                quote!(#name: (style.#name != default.#name).then_some(style.#name),)
            } else {
                quote!(#name: (style.#name != default.#name).then(|| style.#name.clone()),)
            }
        })
        .collect();
    let apply: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            if is_copy(spec.ty) {
                quote! {
                    if let Some(value) = patch.#name {
                        self.#name = value;
                    }
                }
            } else {
                quote! {
                    if let Some(value) = &patch.#name {
                        self.#name = value.clone();
                    }
                }
            }
        })
        .collect();
    quote! {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutStyle {
            #(#style_fields)*
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutPatch {
            #(#patch_fields)*
        }

        impl LayoutPatch {
            pub const DEFAULT: Self = Self {
                #(#defaults)*
            };

            pub fn is_empty(&self) -> bool {
                *self == Self::DEFAULT
            }
        }

        impl From<&LayoutStyle> for LayoutPatch {
            fn from(style: &LayoutStyle) -> Self {
                let default = LayoutStyle::default();
                Self {
                    #(#diff)*
                }
            }
        }

        impl LayoutStyle {
            pub fn apply(&mut self, patch: &LayoutPatch) {
                #(#apply)*
            }

            pub fn row_gap(&self) -> u32 {
                if self.row_gap == 0 { self.gap } else { self.row_gap }
            }

            pub fn column_gap(&self) -> u32 {
                if self.column_gap == 0 { self.gap } else { self.column_gap }
            }
        }
    }
}

fn widget() -> TokenStream {
    let specs = fields(StyleOwner::Widget);
    let style_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: #ty,)
        })
        .collect();
    let defaults: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let default = default_expr(spec);
            quote!(#name: #default,)
        })
        .collect();

    let patch_list = patches(StyleOwner::Widget);
    let patch_fields: Vec<_> = patch_list
        .iter()
        .map(|patch| {
            let spec = specs
                .iter()
                .find(|spec| {
                    spec.patches
                        .iter()
                        .any(|candidate| candidate.name == patch.name)
                })
                .expect("every patch belongs to a field of its owner");
            let name = ident(patch.name);
            let ty = patch_field_type(spec, patch);
            quote!(pub #name: Option<#ty>,)
        })
        .collect();
    let patch_defaults: Vec<_> = patch_list
        .iter()
        .map(|patch| {
            let name = ident(patch.name);
            quote!(#name: None,)
        })
        .collect();

    let apply: Vec<_> = specs
        .iter()
        .flat_map(|spec| {
            spec.patches
                .iter()
                .map(move |patch| apply_line(spec, patch))
        })
        .collect();
    let merge: Vec<_> = patch_list.iter().map(|patch| merge_line(patch)).collect();

    let transitions = fields(StyleOwner::Transition);
    let transition_fields: Vec<_> = transitions
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            quote!(pub #name: Option<Duration>,)
        })
        .collect();
    let transition_defaults: Vec<_> = transitions
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            quote!(#name: None,)
        })
        .collect();
    let transition_merge: Vec<_> = patches(StyleOwner::Transition)
        .iter()
        .map(|patch| merge_line(patch))
        .collect();

    let tracks: Vec<_> = specs
        .iter()
        .flat_map(|spec| {
            spec.patches.iter().filter_map(move |patch| {
                let tween = patch_tween(spec, patch);
                (tween != TweenKind::None).then_some((spec, patch, tween))
            })
        })
        .collect();
    let track_fields: Vec<_> = tracks
        .iter()
        .map(|(_, patch, ..)| {
            let name = ident(patch.name);
            quote!(#name: Option<Track>,)
        })
        .collect();
    let track_new: Vec<_> = tracks
        .iter()
        .map(|(_, patch, ..)| {
            let name = ident(patch.name);
            quote!(#name: durations.#name.map(Track::new),)
        })
        .collect();
    let track_style: Vec<_> = tracks
        .iter()
        .map(|(spec, patch, tween)| {
            let name = ident(patch.name);
            let target = patch_target(spec, patch);
            let value = tween_expr(*tween, &target);
            quote! {
                if let Some(track) = self.#name {
                    style.#target = #value;
                }
            }
        })
        .collect();
    let track_advance: Vec<_> = tracks
        .iter()
        .map(|(_, patch, ..)| {
            let name = ident(patch.name);
            quote!(&mut self.#name,)
        })
        .collect();

    quote! {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct WidgetStyle {
            #(#style_fields)*
        }

        impl Default for WidgetStyle {
            fn default() -> Self {
                Self {
                    #(#defaults)*
                }
            }
        }

        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct WidgetStylePatch {
            #(#patch_fields)*
        }

        impl WidgetStylePatch {
            pub const DEFAULT: Self = Self {
                #(#patch_defaults)*
            };

            pub(crate) fn merge(&mut self, other: &Self) {
                #(#merge)*
            }
        }

        impl WidgetStyle {
            fn apply(&mut self, patch: &WidgetStylePatch) {
                #(#apply)*
            }
        }

        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub struct Transitions {
            #(#transition_fields)*
        }

        impl Transitions {
            pub const DEFAULT: Self = Self {
                #(#transition_defaults)*
            };

            pub fn is_empty(&self) -> bool {
                *self == Self::DEFAULT
            }

            pub(crate) fn merge(&mut self, other: &Self) {
                #(#transition_merge)*
            }
        }

        pub(super) struct Transition {
            from: WidgetStyle,
            pub(super) to: WidgetStyle,
            #(#track_fields)*
        }

        impl Transition {
            pub(super) fn new(from: WidgetStyle, to: WidgetStyle, durations: Transitions) -> Self {
                Self {
                    from,
                    to,
                    #(#track_new)*
                }
            }

            pub(super) fn style(&self) -> WidgetStyle {
                let mut style = self.to;
                #(#track_style)*
                style
            }

            pub(super) fn advance(&mut self, delta: Duration) -> bool {
                let tracks = [#(#track_advance)*];
                let mut running = false;
                for track in tracks.into_iter().flatten() {
                    running |= track.advance(delta);
                }
                running
            }
        }
    }
}

fn text() -> TokenStream {
    let specs = fields(StyleOwner::Text);
    let style_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: #ty,)
        })
        .collect();
    let new_fields: Vec<_> = specs
        .iter()
        .map(|spec| match spec.name {
            "font_size" => quote!(font_size,),
            "line_height" => quote!(line_height: font_size * 1.2,),
            _ => {
                let name = ident(spec.name);
                let default = default_expr(spec);
                quote!(#name: #default,)
            }
        })
        .collect();
    let apply: Vec<_> = specs
        .iter()
        .flat_map(|spec| {
            spec.patches
                .iter()
                .map(move |patch| apply_line(spec, patch))
        })
        .collect();
    let patch_list = patches(StyleOwner::Text);
    let patch_fields: Vec<_> = patch_list
        .iter()
        .map(|patch| {
            let spec = specs
                .iter()
                .find(|spec| {
                    spec.patches
                        .iter()
                        .any(|candidate| candidate.name == patch.name)
                })
                .expect("every patch belongs to a field of its owner");
            let name = ident(patch.name);
            let ty = patch_field_type(spec, patch);
            quote!(pub #name: Option<#ty>,)
        })
        .collect();
    let patch_defaults: Vec<_> = patch_list
        .iter()
        .map(|patch| {
            let name = ident(patch.name);
            quote!(#name: None,)
        })
        .collect();
    let merge: Vec<_> = patch_list.iter().map(|patch| merge_line(patch)).collect();

    quote! {
        #[derive(Clone, Debug, PartialEq)]
        pub struct TextStyle {
            #(#style_fields)*
        }

        impl TextStyle {
            pub fn new(font_size: f32) -> Self {
                Self {
                    #(#new_fields)*
                }
            }

            pub(crate) fn apply(&mut self, patch: &TextStylePatch) {
                #(#apply)*
            }
        }

        impl Default for TextStyle {
            fn default() -> Self {
                Self::new(14.0)
            }
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct TextStylePatch {
            #(#patch_fields)*
        }

        impl TextStylePatch {
            pub const DEFAULT: Self = Self {
                #(#patch_defaults)*
            };

            pub(crate) fn merge(&mut self, other: &Self) {
                #(#merge)*
            }
        }
    }
}

fn codegen_fields() -> TokenStream {
    let emit = |owner, name, patch_type| {
        let fields: Vec<_> = patches(owner)
            .iter()
            .map(|patch| {
                let field = ident(patch.name);
                match patch.converter {
                    Some(converter) => {
                        let converter = ident(converter);
                        quote!(#field => convert::#converter,)
                    }
                    None => quote!(#field,),
                }
            })
            .collect();
        let name = ident(name);
        quote! {
            fn #name(patch: &#patch_type) -> String {
                let mut out = String::new();
                fields!(out, patch, #patch_type, { #(#fields)* });
                out
            }
        }
    };
    let layout = emit(StyleOwner::Layout, "layout", quote!(LayoutPatch));
    let widget = emit(StyleOwner::Widget, "widget_patch", quote!(WidgetStylePatch));
    let text = emit(StyleOwner::Text, "text_patch", quote!(TextStylePatch));
    let transitions = emit(StyleOwner::Transition, "transitions", quote!(Transitions));
    quote! {
        #layout
        #widget
        #text
        #transitions
    }
}

fn codegen() -> TokenStream {
    let mut arms = Vec::new();
    let mut seen = HashSet::new();
    for spec in PROPERTIES {
        let (name, sided) = variant(spec.key);
        if !seen.insert(name.to_string()) {
            continue;
        }
        let arm = if sided {
            format!("Kind::{name}(side) => format!(\"s::Kind::{name}(s::Side::{{side:?}})\"),")
        } else {
            format!("Kind::{name} => \"s::Kind::{name}\".into(),")
        };
        arms.push(
            arm.parse::<TokenStream>()
                .expect("generated match arm parses"),
        );
    }
    quote! {
        fn kind(value: Kind) -> String {
            match value {
                #(#arms)*
            }
        }
    }
}

fn fields(owner: StyleOwner) -> Vec<&'static FieldSpec> {
    FIELDS.iter().filter(|spec| spec.owner == owner).collect()
}

fn patches(owner: StyleOwner) -> Vec<&'static PatchField> {
    let mut seen = HashSet::new();
    let mut result: Vec<_> = fields(owner)
        .iter()
        .flat_map(|spec| spec.patches.iter())
        .filter(|patch| seen.insert(patch.name))
        .collect();
    result.sort_by_key(|patch| patch.order);
    result
}

fn ident(name: &str) -> proc_macro2::Ident {
    format_ident!("{name}")
}

fn side_ident(side: Side) -> proc_macro2::Ident {
    format_ident!("{}", format!("{side:?}").to_lowercase())
}

fn side_variant(side: Side) -> proc_macro2::Ident {
    format_ident!("{side:?}")
}

fn field_type(ty: FieldType) -> TokenStream {
    match ty {
        FieldType::Dimension => quote!(Dimension),
        FieldType::U32 => quote!(u32),
        FieldType::F32 => quote!(f32),
        FieldType::Bool => quote!(bool),
        FieldType::SidesU32 => quote!(Sides<u32>),
        FieldType::Display => quote!(Display),
        FieldType::Direction => quote!(Direction),
        FieldType::Justify => quote!(Justify),
        FieldType::Align => quote!(Align),
        FieldType::TrackList => quote!(Vec<TrackSize>),
        FieldType::GridLine => quote!(GridLine),
        FieldType::GridAutoFlow => quote!(GridAutoFlow),
        FieldType::AreaTemplate => quote!(AreaTemplate),
        FieldType::String => quote!(String),
        FieldType::Paint => quote!(Paint),
        FieldType::Color => quote!(Color),
        FieldType::SidesColor => quote!(BorderColors),
        FieldType::Shadow => quote!(Shadow),
        FieldType::VerticalAlign => quote!(VerticalAlign),
        FieldType::OptionColor => quote!(Option<Color>),
        FieldType::OptionString => quote!(Option<String>),
        FieldType::TextAlign => quote!(TextAlign),
        FieldType::U16 => quote!(u16),
        FieldType::Duration => quote!(Duration),
    }
}

fn is_copy(ty: FieldType) -> bool {
    !matches!(
        ty,
        FieldType::TrackList | FieldType::AreaTemplate | FieldType::String
    )
}

fn default_expr(spec: &FieldSpec) -> TokenStream {
    match spec.default {
        FieldDefault::Default => quote!(Default::default()),
        FieldDefault::Zero => match spec.ty {
            FieldType::F32 => quote!(0.0),
            _ => quote!(0),
        },
        FieldDefault::One => quote!(1.0),
        FieldDefault::None => quote!(None),
        FieldDefault::Text => quote!(Color::rgb(15, 23, 42)),
        FieldDefault::FontSize => quote!(14.0),
        FieldDefault::LineHeight => quote!(14.0 * 1.2),
        FieldDefault::Weight => quote!(400),
        FieldDefault::TransparentSides => quote!(BorderColors::all(Color::TRANSPARENT)),
    }
}

fn patch_field_type(spec: &FieldSpec, patch: &PatchField) -> TokenStream {
    match patch.target {
        PatchTarget::This => match spec.ty {
            FieldType::OptionColor => quote!(Color),
            FieldType::OptionString => quote!(String),
            ty => field_type(ty),
        },
        PatchTarget::Side(_) => quote!(Color),
        PatchTarget::Subfield("x") | PatchTarget::Subfield("y") => quote!(i32),
        PatchTarget::Subfield("blur") => quote!(u32),
        PatchTarget::Subfield(_) => quote!(Color),
        PatchTarget::Scaled(_) => quote!(f32),
    }
}

fn patch_tween(spec: &FieldSpec, patch: &PatchField) -> TweenKind {
    if spec.tween == TweenKind::None {
        return TweenKind::None;
    }
    match (spec.ty, patch.target) {
        (FieldType::Paint, _) => TweenKind::Paint,
        (FieldType::OptionColor, _) => TweenKind::OptionColor,
        (FieldType::F32, PatchTarget::This) => TweenKind::Float,
        (FieldType::Color, _) => TweenKind::Color,
        (FieldType::SidesColor, PatchTarget::Side(_)) => TweenKind::Color,
        (FieldType::Shadow, PatchTarget::Subfield("color")) => TweenKind::Color,
        _ => TweenKind::None,
    }
}

fn patch_target(spec: &FieldSpec, patch: &PatchField) -> TokenStream {
    let field = ident(spec.name);
    match patch.target {
        PatchTarget::This => quote!(#field),
        PatchTarget::Side(side) => {
            let side = side_ident(side);
            quote!(#field.#side)
        }
        PatchTarget::Subfield(name) => {
            let name = ident(name);
            quote!(#field.#name)
        }
        PatchTarget::Scaled(_) => quote!(#field),
    }
}

fn tween_expr(tween: TweenKind, target: &TokenStream) -> TokenStream {
    match tween {
        TweenKind::Paint => quote!(tween_paint(self.from.#target, self.to.#target, track.value())),
        TweenKind::OptionColor => {
            quote!(tween_tint(self.from.#target, self.to.#target, track.value()))
        }
        TweenKind::Color | TweenKind::Float => {
            quote!(self.from.#target.tween(self.to.#target, track.value()))
        }
        TweenKind::None => unreachable!("only tweenable fields reach tween_expr"),
    }
}

fn apply_line(spec: &FieldSpec, patch: &PatchField) -> TokenStream {
    let name = ident(patch.name);
    let target = patch_target(spec, patch);
    match patch.target {
        PatchTarget::Scaled(from) => {
            let from = ident(from);
            quote! {
                if let Some(value) = patch.#from {
                    self.#target = value * 1.2;
                }
            }
        }
        PatchTarget::This if spec.ty == FieldType::OptionString => quote! {
            if let Some(value) = &patch.#name {
                self.#target = Some(value.clone());
            }
        },
        PatchTarget::This if spec.ty == FieldType::OptionColor => quote! {
            if let Some(value) = patch.#name {
                self.#target = Some(value);
            }
        },
        _ => quote! {
            if let Some(value) = patch.#name {
                self.#target = value;
            }
        },
    }
}

fn owned_patch(patch: &PatchField) -> bool {
    FIELDS.iter().any(|spec| {
        spec.ty == FieldType::OptionString
            && spec
                .patches
                .iter()
                .any(|candidate| candidate.name == patch.name)
    })
}

fn merge_line(patch: &PatchField) -> TokenStream {
    let name = ident(patch.name);
    if owned_patch(patch) {
        quote! {
            if other.#name.is_some() {
                self.#name = other.#name.clone();
            }
        }
    } else {
        quote! {
            if other.#name.is_some() {
                self.#name = other.#name;
            }
        }
    }
}

fn spec(spec: &PropertySpec) -> TokenStream {
    let kind = kind(spec.key);
    let name = spec.name;
    let ty = ty(&spec.ty);
    let state = state(spec.in_state);
    let target = target(spec.target);
    let animatable = spec.animatable;
    quote! {
        Spec {
            kind: #kind,
            name: #name,
            ty: #ty,
            in_state: #state,
            target: #target,
            animatable: #animatable,
        }
    }
}

fn variant(key: PropertyKey) -> (proc_macro2::Ident, bool) {
    match key {
        PropertyKey::PaddingSide(_) => (format_ident!("PaddingSide"), true),
        PropertyKey::BorderWidthSide(_) => (format_ident!("BorderWidthSide"), true),
        PropertyKey::BorderColorSide(_) => (format_ident!("BorderColorSide"), true),
        key => (format_ident!("{key:?}"), false),
    }
}

fn kind(key: PropertyKey) -> TokenStream {
    let (name, sided) = variant(key);
    if sided {
        let side = side_variant(match key {
            PropertyKey::PaddingSide(side)
            | PropertyKey::BorderWidthSide(side)
            | PropertyKey::BorderColorSide(side) => side,
            _ => unreachable!("only three property keys are sided"),
        });
        quote!(Kind::#name(Side::#side))
    } else {
        quote!(Kind::#name)
    }
}

fn ty(ty: &Type) -> TokenStream {
    match ty {
        Type::Keywords(names) => quote!(Type::Keywords(&[#(#names),*])),
        Type::NumberRange(from, to) => quote!(Type::NumberRange(#from, #to)),
        Type::NumberIn(numbers) => quote!(Type::NumberIn(&[#(#numbers),*])),
        ty => {
            let name = format_ident!("{ty:?}");
            quote!(Type::#name)
        }
    }
}

fn state(policy: StatePolicy) -> TokenStream {
    let name = format_ident!("{policy:?}");
    quote!(StatePolicy::#name)
}

fn target(target: Target) -> TokenStream {
    match target {
        Target::Layout(field) => {
            let field = layout_field(field);
            quote!(Target::Layout(#field))
        }
        Target::Style(field) => {
            let field = style_field(field);
            quote!(Target::Style(#field))
        }
        Target::Text(field) => {
            let name = format_ident!("{field:?}");
            quote!(Target::Text(TextField::#name))
        }
    }
}

fn layout_field(field: LayoutField) -> TokenStream {
    match field {
        LayoutField::Padding(side) => {
            let side = side_variant(side);
            quote!(LayoutField::Padding(Side::#side))
        }
        LayoutField::BorderWidthSide(side) => {
            let side = side_variant(side);
            quote!(LayoutField::BorderWidthSide(Side::#side))
        }
        field => {
            let name = format_ident!("{field:?}");
            quote!(LayoutField::#name)
        }
    }
}

fn style_field(field: StyleField) -> TokenStream {
    match field {
        StyleField::BorderSide(side) => {
            let side = side_variant(side);
            quote!(StyleField::BorderSide(Side::#side))
        }
        field => {
            let name = format_ident!("{field:?}");
            quote!(StyleField::#name)
        }
    }
}

fn parser(parser: ShorthandParser) -> proc_macro2::Ident {
    match parser {
        ShorthandParser::Box => format_ident!("box_values"),
        ShorthandParser::Border => format_ident!("border_values"),
        ShorthandParser::BorderColor => format_ident!("color_values"),
        ShorthandParser::Shadow => format_ident!("shadow_values"),
    }
}
