fn rename(host: Host, name: String) -> Host {
    Host { name, ..{ host } }
}
