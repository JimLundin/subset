use syn::visit::Visit;

pub struct Names {
    pub found: Vec<String>,
}

impl Names {
    fn add(&mut self, name: String) {
        self.found.push(name);
    }
}

impl<'ast> Visit<'ast> for Names {
    fn visit_ident(&mut self, i: &'ast syn::Ident) {
        self.add(i.to_string());
    }
}

pub fn names(file: &syn::File) -> Vec<String> {
    let mut names = Names { found: Vec::new() };
    names.visit_file(file);
    names.found
}
