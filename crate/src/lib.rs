use cfg_if::cfg_if;
use wasm_bindgen;
use web_sys;
use wasm_bindgen::prelude::*;
use ra_syntax::{ast, AstNode, TreeArc};
use ra_mbe::{MacroRules, ast_to_token_tree, token_tree_to_macro_stmts, token_tree_to_macro_items};
use ra_tt::Subtree;
use serde_derive::Serialize;
use std::collections::HashMap;

cfg_if! {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        fn set_panic_hook() {}
    }
}

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

#[derive(Serialize)]
pub struct MacroExpansionText {
    call_site: String,
    expanded: String,
    children: Vec<MacroExpansionText>,
}

#[derive(Debug, Clone)]
enum ExpansionKind {
    Items(TreeArc<ast::MacroItems>),
    Stmts(TreeArc<ast::MacroStmts>),
    Unknown,
}

#[derive(Debug, Clone)]
struct MacroExpansion {
    call_site: TreeArc<ast::MacroCall>,
    expanded: ra_tt::Subtree,
    kind: ExpansionKind,

    children: Vec<MacroExpansion>,
}

impl<'a> MacroExpansion {
    fn to_textual(&self) -> MacroExpansionText {
        MacroExpansionText {
            call_site: self.call_site.syntax().text().to_string(),
            expanded: self.syntax().map(|x|x.text().to_string()).unwrap_or_else(||{
                self.expanded.to_string()
            }),

            children: self.children.iter().map(|c|c.to_textual()).collect(),
        }
    }

    fn syntax(&self) -> Option<&ra_syntax::SyntaxNode> {
        match &self.kind {
            ExpansionKind::Unknown => None,
            ExpansionKind::Items(items) => Some(items.syntax()),
            ExpansionKind::Stmts(stmts) => Some(stmts.syntax()),
        }
    }
}


#[derive(Serialize)]
pub struct ParsedData {
    syntax_nodes: String,
    macro_rules: Vec<String>,
    calls: Vec<MacroExpansionText>,
}

#[allow(unused)]
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => {
        web_sys::console::log_1(&format_args!($($t)*).to_string().into())
    }
}

#[derive(Debug)]
struct MacroDef {
    name: String,
    subtree: Subtree,
    rules: MacroRules,
}

impl MacroDef {
    fn to_string(&self) -> String {
        format!("{} {}", self.name, self.subtree.to_string())
    }
}

fn get_macro_rules(macro_call: &ast::MacroCall) -> Option<MacroDef> {
    if macro_call.path()?.segment()?.name_ref()?.text() != "macro_rules" {
        return None;
    }
    
    let rule = macro_call.token_tree()?;
    let name = macro_call.syntax().children().find_map(ast::Name::cast)?.text().to_string();

    let subtree = ast_to_token_tree(rule)?.0;
    let rules = MacroRules::parse(&subtree).ok()?;

    Some(MacroDef {
        name,
        subtree,
        rules,
    })    
}

fn expand<'a>(macro_call: &'a ast::MacroCall, defs: &HashMap<String, MacroDef>) -> Option<MacroExpansion> {
    let name = macro_call.path()?.segment()?.name_ref()?.text();
    let def = defs.get(&name.to_string())?;
    let subtree = ast_to_token_tree(macro_call.token_tree()?)?.0;
    let expanded = def.rules.expand(&subtree).ok()?;   

    // We don't know which one to use, just try all
    let kind = if let Ok(res) = token_tree_to_macro_stmts(&expanded) {
        ExpansionKind::Stmts(res)        
    } else if let Ok(res) = token_tree_to_macro_items(&expanded) {
        ExpansionKind::Items(res)
    } else {
        ExpansionKind::Unknown
    };
    
    Some(MacroExpansion {
        call_site: macro_call.to_owned(),
        expanded,
        kind,
        children: vec![],
    })
}

#[derive(Default)]
struct ResolveContext {
    rules: HashMap<String, MacroDef>,
}

fn resolve_macros(node: &ra_syntax::SyntaxNode, ctx: &mut ResolveContext, resolveds: &mut Vec<MacroExpansion>, recursive: bool) {
    let mut calls = Vec::new();

    let macro_calls = node.descendants().filter_map(ast::MacroCall::cast);
    for mc in macro_calls {
        if let Some(mc) = get_macro_rules(mc) {
            ctx.rules.insert(mc.name.clone(), mc);
        } else {
            calls.push(mc);
        }
    }

    // Expanding
    let mut calls = calls.into_iter().filter_map(|mc|{
        expand(mc, &ctx.rules)
    }).collect::<Vec<_>>();

    if recursive {        
        for call in calls.iter_mut() {                        
            if let Some(syntax) = call.syntax() {
                resolve_macros(&syntax.to_owned(), ctx, &mut call.children,recursive);
            }            
        }
    }

    resolveds.extend(calls);
}


#[wasm_bindgen]
pub fn parse_text_to_syntax_node(s: String, recursive:bool) -> JsValue  {
    let src = ast::SourceFile::parse(&s);

    let mut ctx = ResolveContext::default(); 
    let mut calls = vec![];

    resolve_macros(src.syntax(), &mut ctx, &mut calls, recursive);

    let syntax_nodes = src.syntax().debug_dump().to_string();

    let rules = ctx.rules.iter().map(|(_, rule)|rule.to_string()).collect();

    JsValue::from_serde(&ParsedData {
        syntax_nodes, 
        macro_rules : rules,
        calls: calls.iter().map(|c|c.to_textual()).collect(),
    }).unwrap()    
}

// Called by our JS entry point to run the example
#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    // If the `console_error_panic_hook` feature is enabled this will set a panic hook, otherwise
    // it will do nothing.
    set_panic_hook();

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Web based mbe!!");

    body.append_child(&val)?;

    Ok(())
}

