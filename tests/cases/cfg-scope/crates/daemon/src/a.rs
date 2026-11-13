#[cfg(unix)]
fn socket() -> Option<u8> {
    Some(1)
}

#[cfg(not(unix))]
fn socket() -> Option<u8> {
    None
}

mod paired {
    #[cfg(unix)]
    fn socket() -> Option<u8> {
        Some(1)
    }

    #[cfg(not(unix))]
    fn socket() -> Option<u8> {
        None
    }
}

struct Port(u8);

impl Port {
    #[cfg(unix)]
    fn socket(&self) -> Option<u8> {
        Some(self.0)
    }
}

impl Port {
    #[cfg(not(unix))]
    fn socket(&self) -> Option<u8> {
        None
    }
}

#[cfg(unix)]
impl Port {
    fn value(&self) -> u8 {
        self.0
    }
}

#[cfg(not(unix))]
impl Port {
    fn value(&self) -> u8 {
        self.0
    }
}

fn nested() {
    #[cfg(unix)]
    fn socket() -> Option<u8> {
        Some(1)
    }

    #[cfg(not(unix))]
    fn socket() -> Option<u8> {
        None
    }
}

#[cfg(not(target_family = "wasm"))]
mod native {}
#[cfg(target_family = "wasm")]
mod web {}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(not(unix))]
use std::path::Path;

#[cfg(test)]
pub(crate) use paired_test::check_version;

mod paired_test {
    pub(crate) fn check_version() -> Option<u8> {
        Some(1)
    }
}

mod spellings {
    struct Port(u8);

    impl Port {
        #[cfg(unix)]
        fn socket(&self) -> Option<u8> {
            Some(self.0)
        }
    }

    impl self::Port {
        #[cfg(not(unix))]
        fn socket(&self) -> Option<u8> {
            None
        }
    }

    #[cfg(unix)]
    impl Port {
        fn value(&self) -> u8 {
            self.0
        }
    }

    #[cfg(not(unix))]
    impl self::Port {
        fn value(&self) -> u8 {
            self.0
        }
    }
}

mod generic {
    struct Port<Id> {
        value: Id,
    }

    impl<T> Port<T> {
        #[cfg(unix)]
        fn value(&self) -> &T {
            &self.value
        }
    }

    impl<U> self::Port<U> {
        #[cfg(not(unix))]
        fn value(&self) -> &U {
            &self.value
        }
    }

    #[cfg(unix)]
    impl<T> Port<T> {
        fn read(&self) -> &T {
            &self.value
        }
    }

    #[cfg(not(unix))]
    impl<U> self::Port<U> {
        fn read(&self) -> &U {
            &self.value
        }
    }
}

struct Visitor(u8);

#[cfg(unix)]
impl<'a> syn::visit::Visit<'a> for Visitor {}

#[cfg(not(unix))]
impl<'b> syn::visit::Visit<'b> for self::Visitor {}

mod external_paths {
    struct Visitor(u8);

    #[cfg(unix)]
    impl<'a> syn::visit::Visit<'a> for Visitor {}

    #[cfg(not(unix))]
    impl<'b> ::syn::visit::Visit<'b> for self::Visitor {}

    struct Port<Id> {
        value: Id,
    }

    impl Port<syn::Ident> {
        #[cfg(unix)]
        fn value(&self) -> &syn::Ident {
            &self.value
        }
    }

    impl self::Port<::syn::Ident> {
        #[cfg(not(unix))]
        fn value(&self) -> &::syn::Ident {
            &self.value
        }
    }
}

mod unrelated {
    mod syn {}
}

fn unrelated_block() {
    mod syn {}
}

mod imported_parent {
    use super::Port;

    #[cfg(unix)]
    impl Port {
        fn imported(&self) -> u8 {
            self.0
        }
    }

    #[cfg(not(unix))]
    impl super::Port {
        fn imported(&self) -> u8 {
            self.0
        }
    }
}

mod imported_external {
    use syn::Ident;
    use syn::visit::Visit;

    struct Visitor(u8);

    #[cfg(unix)]
    impl<'a> Visit<'a> for Visitor {}

    #[cfg(not(unix))]
    impl<'b> ::syn::visit::Visit<'b> for self::Visitor {}

    struct Wrapper<Id> {
        value: Id,
    }

    impl Wrapper<Ident> {
        #[cfg(unix)]
        fn value(&self) -> &Ident {
            &self.value
        }
    }

    impl self::Wrapper<syn::Ident> {
        #[cfg(not(unix))]
        fn value(&self) -> &syn::Ident {
            &self.value
        }
    }
}

mod owners {
    pub(crate) struct Port(pub(crate) u8);
}

mod imported_module {
    use super::owners;

    #[cfg(unix)]
    impl owners::Port {
        fn socket(&self) -> Option<u8> {
            Some(self.0)
        }
    }

    #[cfg(not(unix))]
    impl self::owners::Port {
        fn socket(&self) -> Option<u8> {
            None
        }
    }

    impl owners::Port {
        #[cfg(unix)]
        fn connect(&self) -> Option<u8> {
            Some(self.0)
        }
    }

    impl self::owners::Port {
        #[cfg(not(unix))]
        fn connect(&self) -> Option<u8> {
            None
        }
    }
}

fn local_alias() {
    struct LocalPort<Id> {
        value: Id,
    }

    #[cfg(unix)]
    impl<Id> LocalPort<Id> {
        fn socket(&self) -> &Id {
            &self.value
        }
    }

    #[cfg(not(unix))]
    impl<Key> LocalPort<Key> {
        fn socket(&self) -> &Key {
            &self.value
        }
    }
}

struct Pair<First, Second> {
    first: First,
    second: Second,
}

impl<A, B> Pair<A, B> {
    #[cfg(unix)]
    fn first(&self) -> &A {
        &self.first
    }
}

impl<X, Y> Pair<Y, X> {
    #[cfg(not(unix))]
    fn first(&self) -> &Y {
        &self.first
    }
}

mod grouped_self {
    use super::owners::{self, Port};

    #[cfg(unix)]
    impl owners::Port {
        fn grouped(&self) -> Option<u8> {
            Some(self.0)
        }
    }

    #[cfg(not(unix))]
    impl super::owners::Port {
        fn grouped(&self) -> Option<u8> {
            None
        }
    }

    impl Port {
        #[cfg(unix)]
        fn grouped_method(&self) -> Option<u8> {
            Some(self.0)
        }
    }

    impl super::owners::Port {
        #[cfg(not(unix))]
        fn grouped_method(&self) -> Option<u8> {
            None
        }
    }
}

mod raw_name {
    struct r#type(u8);

    #[cfg(unix)]
    impl r#type {
        fn value(&self) -> u8 {
            self.0
        }
    }

    #[cfg(not(unix))]
    impl r#type {
        fn value(&self) -> u8 {
            self.0
        }
    }
}
