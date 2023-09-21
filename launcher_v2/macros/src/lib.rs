/*
Following code is adapted version of macros included in strum crate (https://github.com/Peternator7/strum), distributed under following license:
    MIT License

    Copyright (c) 2019 Peter Glotfelty

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
*/

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Lit};
fn get_variants<const MATCH_ARM: bool>(ast: &DeriveInput) -> syn::Result<Vec<TokenStream>> {
    let name = &ast.ident;
    let mut arms = Vec::new();
    let variants = match &ast.data {
        Data::Enum(v) => &v.variants,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "This macro only supports enums.",
            ))
        }
    };
    let module = if let Some(attr) = ast
        .clone()
        .attrs
        .into_iter()
        .find(|attr| attr.path().is_ident("module"))
    {
        let mut s = attr.meta.require_list().unwrap().tokens.to_string();
        s.push('.');
        s
    } else {
        String::new()
    };

    for variant in variants {
        let ident = &variant.ident;

        let output = [&module, &name.to_string(), ".", &ident.to_string(), ""].join("");
        let output_lit = Lit::new(Literal::string(&output));
        let output = if let Lit::Str(output_strlit) = output_lit {
            output_strlit
        } else {
            unreachable!()
        };

        let params = match variant.fields {
            Fields::Unit => quote! {},
            Fields::Unnamed(..) => quote! { (..) },
            Fields::Named(..) => quote! { {..} },
        };
        if MATCH_ARM {
            arms.push(quote! {#name::#ident #params =>  ::rust_i18n::t!(#output ) });
        } else {
            arms.push(quote! {::rust_i18n::t!(#output ) });
        }
    }

    Ok(arms)
}

////////////////////////////////////////////////////////////////////

fn walk_fields(ast: &DeriveInput) -> syn::Result<Vec<TokenStream>> {
    let name = &ast.ident;
    let mut arms = Vec::new();
    let fields = match &ast.data {
        Data::Struct(v) => &v.fields,
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "This macro only supports structs.",
            ))
        }
    };
    let module = if let Some(attr) = ast
        .clone()
        .attrs
        .into_iter()
        .find(|attr| attr.path().is_ident("module"))
    {
        if let Ok(ident) = attr.parse_args::<proc_macro2::Ident>() {
            let mut s = ident.to_string();
            s.push('.');
            s
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    for field in fields {
        let ident = &field.ident;
        if field.attrs.clone().into_iter().any(|f| {
            if let Some(id) = f.meta.path().get_ident() {
                id.to_string() == "skip"
            } else {
                false
            }
        }) {
            continue;
        }

        let ident_str = if let Some(id) = ident {
            id.to_string()
        } else {
            String::default()
        };

        let output = [&module, &name.to_string(), ".", &ident_str].join("");
        let output_lit = Lit::new(Literal::string(&output));
        let output = if let Lit::Str(output_strlit) = output_lit {
            output_strlit
        } else {
            unreachable!()
        };

        arms.push(quote! { self. #ident .show_ui( ui, ::rust_i18n::t!( #output).as_ref() ) });
    }

    if arms.len() < fields.len() {
        arms.push(quote! { false});
    }

    Ok(arms)
}

fn display_gui_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let collapsed = ast
        .clone()
        .attrs
        .into_iter()
        .find(|attr| attr.path().is_ident("uncollapsed"))
        .is_none();
    let fields_code = walk_fields(ast)?;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    if collapsed {
        Ok(quote! {
            impl #impl_generics DisplayGUI for #name #ty_generics #where_clause {
                fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
                    let mut ret=false;
                    ::egui::CollapsingHeader::new(label).default_open(true).show(ui, |ui|{
                        ::egui::Grid::new(ui.next_auto_id()).show(ui,|ui|
                        {
                            #(ret |= #fields_code; ui.end_row();)*
                        })
                    });
                    ret
                }
            }
        })
    } else {
        Ok(quote! {
            impl #impl_generics DisplayGUI for #name #ty_generics #where_clause {
                fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
                        let mut ret=false;
                        #(ret |= #fields_code;)*
                        ret
                }
            }
        })
    }
}
#[proc_macro_derive(DisplayGUI, attributes(skip, module, uncollapsed))]
pub fn display_gui(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);

    let toks = display_gui_inner(&ast).unwrap_or_else(|err| err.to_compile_error());
    debug_print_generated(&ast, &toks);
    toks.into()
}
////////////////////////////////////////

fn enum_combobox_i18n_inner(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let variants = get_variants::<false>(ast)?;
    let variants_count = variants.len();
    Ok(quote! {
        impl #impl_generics DisplayGUI for #name #ty_generics #where_clause {
            fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
                ::lazy_static::lazy_static! {
                    static ref VARIANTS_I18N: ::std::sync::Mutex<[String; #variants_count]> = ::std::default::Default::default();
                }
                ::lazy_static::lazy_static! {
                    static ref CACHED_LOCALE: ::std::sync::Mutex<String> = ::std::sync::Mutex::new(String::new());
                }
                let mut cached_locale=CACHED_LOCALE.lock().unwrap();
                let mut variants_i18n=VARIANTS_I18N.lock().unwrap();
                if *cached_locale != ::rust_i18n::locale()
                {
                    *variants_i18n=[ #(#variants),*];
                    *cached_locale=::rust_i18n::locale();
                }
                let mut idx = *self as usize;
                ::egui::Label::new(label).ui(ui);
                ::egui::ComboBox::from_id_source(ui.next_auto_id()).show_index(
                    ui,
                    &mut idx,
                    #variants_count,
                    |i| &variants_i18n[i],
                );
                if idx != *self as usize {
                    *self = Self::from_repr(idx).unwrap();
                    return true;
                }
                return false;
            }
        }
    })
}

/// requires derive(strum::FromRepr)
#[proc_macro_derive(EnumComboboxI18N, attributes(module))]
pub fn enum_combobox_i18n(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);

    let toks = enum_combobox_i18n_inner(&ast).unwrap_or_else(|err| err.to_compile_error());
    debug_print_generated(&ast, &toks);
    toks.into()
}
////////////////////////////////////////

fn debug_print_generated(ast: &DeriveInput, toks: &TokenStream) {
    let debug = std::env::var("STRUM_DEBUG");
    if let Ok(s) = debug {
        if s == "1" {
            println!("{}", toks);
        }

        if ast.ident == s {
            println!("{}", toks);
        }
    }
}
