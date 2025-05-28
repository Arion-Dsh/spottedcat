
use proc_macro::TokenStream;
use quote::{quote};
use syn::{
    DeriveInput, 
    Data, 
    Ident,
    Field,
    FieldsNamed,
    DataStruct,
};

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = syn::parse_macro_input!(item as DeriveInput);
    impl_component(&mut ast)
}

fn impl_component(ast: &mut syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let inner_name = Ident::new(&format!("{}Inner", name), name.span());

    let has_derive_default = ast.attrs.iter()
        .filter(|attr| attr.path.is_ident("derive"))
        .flat_map(|attr| match attr.parse_meta() {
            Ok(syn::Meta::List(list)) => Some(list.nested.into_iter().collect::<Vec<_>>()),
            _ => None
        })
        .flatten()
        .any(|nested_meta| {
            if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = nested_meta {
                path.is_ident("Default")
            } else {
                false
            }
        });

    if !has_derive_default {
        return syn::Error::new_spanned(
            ast,
            "#[component] requires #[derive(Default)]",
        )
        .to_compile_error()
        .into();
    }

    // Get fields from the struct
    let fields = match &ast.data {
        syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(fields), .. }) => {
            &fields.named
        }
        _ => {
            return syn::Error::new_spanned(
                ast,
                "#[component] can only be used on structs with named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate inner struct fields
    let inner_fields = fields.iter().map(|f| {
        let Field { ident, ty, .. } = f;
        quote! {
            pub #ident: #ty
        }
    });

    // Generate getter and setter methods
    let getter_setter_methods = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let get_name = Ident::new(&format!("get_{}", name), name.span());
        let set_name = Ident::new(&format!("set_{}", name), name.span());

        quote! {
            pub fn #get_name(&self) -> Result<#ty, std::cell::BorrowError> {
                match self.inner.try_borrow() {
                    Ok(borrowed) => Ok(borrowed.#name.clone()),
                    Err(e) => Err(e)
                }
            }
            pub fn #set_name(&self, value: #ty) -> Result<(), std::cell::BorrowMutError> {
                match self.inner.try_borrow_mut() {
                    Ok(mut borrowed) => {
                        borrowed.#name = value;
                        Ok(())
                    }
                    Err(e) => Err(e)
                }
            }
        }
    });

    // Generate the implementation
    let attrs = ast.attrs.iter()
        .filter(|attr| !attr.path.is_ident("component"))
        .cloned()
        .collect::<Vec<_>>();
    
    let expanded = quote! {
        #(#attrs)*
        pub struct #inner_name {
            #(#inner_fields),*
        }

        #(#attrs)*
        pub struct #name {
            inner: std::sync::Arc<std::cell::RefCell<#inner_name>>,
        }

        impl #name {
            pub fn new() -> Self {
                let inner = #inner_name::default();
                Self {
                    inner: std::sync::Arc::new(std::cell::RefCell::new(inner)),
                }
            }

            #(#getter_setter_methods)*
        }

        impl Component for #name {
            fn get_type_id(&self) -> std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }
        }
    };
    
    expanded.into()
}
