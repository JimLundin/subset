mod first {
    #[cfg(unix)]
    fn socket() -> Option<u8> {
        Some(1)
    }
}

mod second {
    #[cfg(not(unix))]
    fn socket() -> Option<u8> {
        None
    }
}

struct First(u8);
struct Second(u8);

impl First {
    #[cfg(unix)]
    fn connect(&self) -> Option<u8> {
        Some(self.0)
    }
}

impl Second {
    #[cfg(not(unix))]
    fn connect(&self) -> Option<u8> {
        None
    }
}

fn outer_first() {
    #[cfg(unix)]
    fn bind() -> Option<u8> {
        Some(1)
    }
}

fn outer_second() {
    #[cfg(not(unix))]
    fn bind() -> Option<u8> {
        None
    }
}

mod owner_first {
    pub(crate) struct Port(pub(crate) u8);
}

mod owner_second {
    pub(crate) struct Port(pub(crate) u8);
}

#[cfg(unix)]
impl owner_first::Port {
    fn value(&self) -> u8 {
        self.0
    }
}

#[cfg(not(unix))]
impl owner_second::Port {
    fn value(&self) -> u8 {
        self.0
    }
}

struct Port(u8);

#[cfg(unix)]
fn shared() -> Option<u8> {
    Some(1)
}

impl Port {
    #[cfg(not(unix))]
    fn shared() -> Option<u8> {
        None
    }
}

impl Port {
    fn outer() {
        #[cfg(unix)]
        fn connect() -> Option<u8> {
            Some(1)
        }

        impl Port {
            #[cfg(not(unix))]
            fn connect() -> Option<u8> {
                None
            }
        }
    }
}

struct Generic<Id> {
    value: Id,
}

impl Generic<u8> {
    #[cfg(unix)]
    fn value(&self) -> u8 {
        self.value
    }
}

impl Generic<u16> {
    #[cfg(not(unix))]
    fn value(&self) -> u16 {
        self.value
    }
}

mod bound_vs_concrete {
    struct T(u8);
    struct Port<Id> {
        value: Id,
    }

    impl<T> Port<T> {
        #[cfg(unix)]
        fn value(&self) -> &T {
            &self.value
        }
    }

    impl Port<T> {
        #[cfg(not(unix))]
        fn value(&self) -> &T {
            &self.value
        }
    }
}

struct Resource(u8);

impl Drop for Resource {
    #[cfg(unix)]
    fn drop(&mut self) {}
}

impl Resource {
    #[cfg(not(unix))]
    fn drop(&self) {}
}

fn separate_blocks() {
    {
        #[cfg(unix)]
        fn socket() -> Option<u8> {
            Some(1)
        }
    }
    {
        #[cfg(not(unix))]
        fn socket() -> Option<u8> {
            None
        }
    }
}

mod syn {
    pub(crate) struct Ident(pub(crate) u8);
}

struct Wrapper<Id> {
    value: Id,
}

impl Wrapper<::syn::Ident> {
    #[cfg(unix)]
    fn value(&self) -> &::syn::Ident {
        &self.value
    }
}

impl Wrapper<crate::syn::Ident> {
    #[cfg(not(unix))]
    fn value(&self) -> &crate::syn::Ident {
        &self.value
    }
}
