# supabase-init-schema
A helper script to setup a declarative schema in your Supabase project.

## How To

- copy `/supabase/scripts/init-schema` into your `/supabase/scripts/` directory
- make sure you have `supabase` in your path. If not, adapt the `main.rs` of the script so it can find the executable
- `cd` into `/supabase/scripts/init-schema`, then `cargo run`
- go into your `supabase/config.toml` and adapt the `schema_paths`. In most cases, you will want something like this:
```toml
schema_paths = [
    # 1. setup schemas
    "./schemas/index.sql",
    "./schemas/**/index.sql",

    # 2. setup objects that have no dependencies
    "./schemas/**/enums/**.sql",
    "./schemas/**/types/**.sql",

    # 3. create functions that are required for tables, e.g. when used in default values
    "./schemas/private/functions/my_func.sql",

    # 4. setup tables
    "./schemas/**/tables/**.sql",

    # 5. then the rest of the functions
    "./schemas/**/functions/**.sql",

    # 6. and then the rest
    "./schemas/private/**/*.sql",
    "./schemas/private/**/**/*.sql",
    "./schemas/public/**/*.sql",
    "./schemas/public/**/**/*.sql"
]
```

Run `supabase db diff -f test` to confirm that everything works - it should show no diffs.

> The sample schema in this repository is AI generated. Do not copy from it. It is bad.
