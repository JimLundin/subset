// Comments cost nothing.
enum Light { On, Off }
fn flip(l: Light) -> Light {
    match l { Light::On => Light::Off, Light::Off => Light::On }
}
fn first(v: Option<Light>) {
    if let Some(x) = v { flip(x); }
}
