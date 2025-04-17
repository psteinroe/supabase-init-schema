use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Schema {
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Table {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Function {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct EnablePolicy {
    pub schema: String,
    pub table: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Policy {
    pub schema: String,
    pub name: String,
    pub table: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Index {
    pub schema: String,
    pub name: String,
    pub table: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct View {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct TriggerFunction {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Trigger {
    pub schema: String,
    pub name: String,
    pub table: String,
    pub function: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Enum {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct CompositeType {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Setup {
    pub sql: String,
}

#[derive(Debug)]
pub struct ForeignKey {
    pub constraint_name: String,
    pub source_schema: String,
    pub source_table: String,
    pub target_schema: String,
    pub target_table: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Aggregate {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Operator {
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub struct Sequence {
    pub table: Option<String>,
    pub schema: String,
    pub name: String,
    pub sql: String,
}

#[derive(Debug)]
pub enum StatementLocation {
    Schema(Schema),
    Table(Table),
    Function(Function),
    EnablePolicy(EnablePolicy),
    Policy(Policy),
    Index(Index),
    View(View),
    TriggerFunction(TriggerFunction),
    Trigger(Trigger),
    EnumNode(Enum),
    CompositeType(CompositeType),
    ForeignKey(ForeignKey),
    Setup(Setup),
    Aggregate(Aggregate),
    Operator(Operator),
    Sequence(Sequence),
}

impl StatementLocation {
    pub fn sql(&self) -> String {
        ensure_semicolon(match self {
            StatementLocation::Setup(n) => &n.sql,
            StatementLocation::Schema(n) => &n.sql,
            StatementLocation::Table(n) => &n.sql,
            StatementLocation::Function(n) => &n.sql,
            StatementLocation::EnablePolicy(n) => &n.sql,
            StatementLocation::Policy(n) => &n.sql,
            StatementLocation::Index(n) => &n.sql,
            StatementLocation::View(n) => &n.sql,
            StatementLocation::TriggerFunction(n) => &n.sql,
            StatementLocation::Trigger(n) => &n.sql,
            StatementLocation::EnumNode(n) => &n.sql,
            StatementLocation::CompositeType(n) => &n.sql,
            StatementLocation::ForeignKey(n) => &n.sql,
            StatementLocation::Aggregate(n) => &n.sql,
            StatementLocation::Operator(n) => &n.sql,
            StatementLocation::Sequence(n) => &n.sql,
        })
    }

    pub fn path(&self, base_dir: &Path, nodes: &[StatementLocation]) -> PathBuf {
        match self {
            StatementLocation::Schema(n) => base_dir.join(&n.name).join("index.sql"),
            StatementLocation::Setup(_) => base_dir.join("index.sql"),
            StatementLocation::Table(n) => base_dir
                .join(&n.schema)
                .join("tables")
                .join(format!("{}.sql", n.name)),
            StatementLocation::Function(n) => base_dir
                .join(&n.schema)
                .join("functions")
                .join(format!("{}.sql", n.name)),
            StatementLocation::EnablePolicy(n) => base_dir
                .join(&n.schema)
                .join("policies")
                .join(&n.table)
                .join("enable_rls.sql"),
            StatementLocation::Policy(n) => base_dir
                .join(&n.schema)
                .join("policies")
                .join(&n.table)
                .join(format!("{}.sql", n.name)),
            StatementLocation::Index(n) => base_dir
                .join(&n.schema)
                .join("indices")
                .join(&n.table)
                .join(format!("{}.sql", n.name)),
            StatementLocation::View(n) => base_dir
                .join(&n.schema)
                .join("views")
                .join(format!("{}.sql", n.name)),
            StatementLocation::TriggerFunction(n) => {
                // Find tables that use this trigger function
                let tables: HashSet<_> = nodes
                    .iter()
                    .filter_map(|node| {
                        if let StatementLocation::Trigger(t) = node {
                            if t.function == n.name {
                                Some(t.table.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // If function is used by exactly one table, place it in that table's directory
                // Otherwise, place it in the general triggers directory
                match tables.iter().collect::<Vec<_>>().as_slice() {
                    [table] => base_dir
                        .join(&n.schema)
                        .join("triggers")
                        .join(table)
                        .join(format!("{}.sql", n.name)),
                    _ => base_dir
                        .join(&n.schema)
                        .join("triggers")
                        .join(format!("{}.sql", n.name)),
                }
            }
            StatementLocation::Trigger(n) => base_dir
                .join(&n.schema)
                .join("triggers")
                .join(&n.table)
                .join(format!("{}.sql", n.function)),
            StatementLocation::EnumNode(n) => base_dir
                .join(&n.schema)
                .join("enums")
                .join(format!("{}.sql", n.name)),
            StatementLocation::CompositeType(n) => base_dir
                .join(&n.schema)
                .join("types")
                .join(format!("{}.sql", n.name)),
            StatementLocation::ForeignKey(n) => base_dir
                .join(&n.source_schema)
                .join("fkeys")
                .join(&n.source_table)
                .join(format!("{}.sql", n.constraint_name)),
            StatementLocation::Aggregate(n) => base_dir
                .join(&n.schema)
                .join("aggregates")
                .join(format!("{}.sql", n.name)),
            StatementLocation::Operator(n) => base_dir
                .join(&n.schema)
                .join("operators")
                .join(format!("{}.sql", n.name)),
            StatementLocation::Sequence(n) => {
                let table = n.table.clone().unwrap_or_else(|| {
                    nodes
                        .iter()
                        .filter_map(|node| {
                            if let StatementLocation::Sequence(t) = node {
                                if t.name == n.name && t.schema == n.schema && t.table.is_some() {
                                    Some(t.table.clone().unwrap())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .next()
                        .expect("No table found for sequence")
                });

                base_dir
                    .join(&n.schema)
                    .join("tables")
                    .join(format!("{}.sql", table))
            }
        }
    }
}

fn ensure_semicolon(s: &str) -> String {
    if s.ends_with(';') {
        s.to_string()
    } else {
        format!("{};", s)
    }
}
