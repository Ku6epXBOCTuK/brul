use brul_style_schema::{StyleOwner, TweenKind};
use proc_macro2::TokenStream;
use quote::quote;

use crate::tokens::{
    default_expr, field_type, patch_field_type, patch_target, patch_tween, tween_expr,
};
use crate::util::{
    apply_lines, fields, ident, is_side_patch, merge_lines, patches, side_of, side_variant,
};

pub(crate) fn widget() -> TokenStream {
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

    let apply = apply_lines(&specs);
    let merge = merge_lines(&patch_list);

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
    let transition_patches = patches(StyleOwner::Transition);
    let transition_merge = merge_lines(&transition_patches);

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
    let track_style: Vec<_> = {
        let mut lines: Vec<_> = tracks
            .iter()
            .filter(|(spec, patch, _)| !is_side_patch(spec, patch))
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
        let sides: Vec<_> = tracks
            .iter()
            .filter(|(spec, patch, _)| is_side_patch(spec, patch))
            .collect();
        if !sides.is_empty() {
            let arms: Vec<_> = sides
                .iter()
                .map(|(_, patch, _)| {
                    let side = side_variant(side_of(patch));
                    let name = ident(patch.name);
                    quote!(Side::#side => self.#name,)
                })
                .collect();
            let target = quote!(border[side]);
            let value = tween_expr(TweenKind::Color, &target);
            lines.push(quote! {
                for side in Side::ALL {
                    let Some(track) = (match side {
                        #(#arms)*
                    }) else {
                        continue
                    };
                    style.#target = #value;
                }
            });
        }
        lines
    };
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
