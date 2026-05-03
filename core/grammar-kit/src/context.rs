use std::collections::HashSet;

#[derive(Clone, Default)]
pub struct ScopeStack {
    pub scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    pub fn new() -> Self { Self { scopes: vec![HashSet::new()] } }
    pub fn enter_scope(&mut self) { self.scopes.push(HashSet::new()); }
    pub fn exit_scope(&mut self) { if self.scopes.len() > 1 { self.scopes.pop(); } }
    
    pub fn define(&mut self, name: impl Into<String>) {
        if let Some(scope) = self.scopes.last_mut() { scope.insert(name.into()); }
    }
    
    pub fn is_defined(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

/// Schlanker State für Variablen und Whitespace-Modus.
/// Muss bei Backtracking geklont werden (billig, da meist winzig).
#[derive(Clone)]
pub struct ParseContext {
    pub scopes: ScopeStack,
    pub mode_stack: Vec<bool>, 
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            scopes: ScopeStack::new(),
            mode_stack: Vec::new(),
        }
    }

    pub fn enter_lexical(&mut self) { self.mode_stack.push(true); }
    pub fn exit_mode(&mut self) { self.mode_stack.pop(); }
    pub fn is_lexical(&self) -> bool { *self.mode_stack.last().unwrap_or(&false) }
}

impl Default for ParseContext {
    fn default() -> Self { Self::new() }
}
