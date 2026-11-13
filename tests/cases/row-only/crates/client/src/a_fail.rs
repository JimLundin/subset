struct Config {
    host: &'static str,
    path: &'static str,
}

const UNUSED: &[Config] = &[];
const CONFIG: Config = Config {
    host: "example.com",
    path: "/api",
};

struct Mine {
    marker: &'static str,
}

const MINES: &[Mine] = &[];

fn mine() -> Mine {
    const MINE: Mine = Mine { marker: "local" };
    MINE
}

mod table {
    struct Row {
        marker: &'static str,
    }

    const DEVICES: &[Row] = &[Row { marker: "a" }];
}

mod elsewhere {
    struct Row {
        label: &'static str,
    }
}
