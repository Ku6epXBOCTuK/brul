use super::*;
pub(super) fn expand_effect(definition: EffectDefinition) -> Result<TokenStream2> {
    let name = &definition.name;
    let shader = &definition.shader;
    let animated = definition.animated;
    let ident = &definition.item.ident;
    let syn::Fields::Named(fields) = &definition.item.fields else {
        return Err(syn::Error::new_spanned(
            &definition.item,
            "effect parameters must have named fields",
        ));
    };

    let mut layouts = Vec::new();
    let mut writes = Vec::new();
    let mut slots = 0usize;
    for field in &fields.named {
        let name = field.ident.as_ref().expect("named field");
        let layout = FieldLayout::from_type(&field.ty)?;
        layout.write(name, slots, &mut writes);
        layouts.push((name, layout));
        slots += layout.slots();
    }

    let keys: Vec<_> = layouts.iter().map(|(field, _)| *field).collect();
    let types: Vec<_> = layouts
        .iter()
        .map(|(_, layout)| layout.effect_type())
        .collect();
    let counts: Vec<_> = layouts
        .iter()
        .map(|(_, layout)| layout.slots() as u16)
        .collect();
    let marker = format_ident!("BRUL_EFFECT_{}", constant(&name.value()));
    let mut item = definition.item.clone();
    item.attrs
        .push(syn::parse_quote!(#[derive(Clone, Copy, Debug, Default)]));

    Ok(quote! {
        #item

        impl ::brul::effect::EffectParams for #ident {
            const NAME: &'static str = #name;
            const SHADER: &'static str = include_str!(#shader);
            const ANIMATED: bool = #animated;
            const SLOTS: u16 = #slots as u16;
            const FIELDS: &'static [::brul::effect::EffectField] = &[
                #(
                    ::brul::effect::EffectField {
                        name: stringify!(#keys),
                        ty: #types,
                        slots: #counts,
                    },
                )*
            ];

            fn pack(&self) -> [[f32; 4]; ::brul::effect::EFFECT_SLOTS] {
                const _: () = assert!(#slots <= ::brul::effect::EFFECT_SLOTS);
                let mut slots = [[0.0f32; 4]; ::brul::effect::EFFECT_SLOTS];
                #(#writes)*
                slots
            }
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub const #marker: ::brul::effect::EffectDef =
            <#ident as ::brul::effect::EffectParams>::DEF;
    })
}
