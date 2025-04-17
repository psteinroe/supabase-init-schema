use crate::locations::{
    Aggregate, CompositeType, EnablePolicy, Enum, ForeignKey, Function, Index, Operator, Policy,
    Schema, Sequence, Setup, StatementLocation, Table, Trigger, TriggerFunction, View,
};
use pg_query::protobuf::ObjectType;
use pg_query::{NodeEnum, Node};

pub fn get_nodes(sql: &str) -> Vec<StatementLocation> {
    let mut nodes: Vec<StatementLocation> = Vec::new();

    pg_query::split_with_parser(sql)
        .expect("Failed to parse SQL")
        .iter()
        .for_each(|sql| {
            parse(sql, &mut nodes);
        });

    nodes
}

fn parse(sql: &str, nodes: &mut Vec<StatementLocation>) {
    let node = parse_sql(sql);
    match node {
        pg_query::NodeEnum::CreateSchemaStmt(n) => {
            let schema_name = n.schemaname.to_string();
            nodes.push(StatementLocation::Schema(Schema {
                name: schema_name.clone(),
                sql: sql.to_string(),
            }));
        }
        pg_query::NodeEnum::CommentStmt(c) => match c.objtype() {
            ObjectType::ObjectColumn => {
                let list = &c.object.clone()
                    .expect("Missing object in column comment")
                    .node
                    .expect("Missing node in column comment object");

                if let NodeEnum::List(l) = list {
                    let items = extract_names(&l.items, "column comment");
                    validate_item_count(&items, 3, "column comment");

                    let schema = &items[0];
                    let table_name = &items[1];
                    let column_name = &items[2];

                    if find_table(nodes, schema, table_name) {
                        nodes.push(StatementLocation::Table(Table {
                            name: table_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON COLUMN \"{}\".\"{}\".\"{}\" IS E'{}';",
                                schema, table_name, column_name, c.comment.replace("'", "''")
                            ),
                        }));
                    } else if find_view(nodes, schema, table_name) {
                        nodes.push(StatementLocation::View(View {
                            name: table_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON COLUMN \"{}\".\"{}\".\"{}\" IS E'{}';",
                                schema, table_name, column_name, c.comment.replace("'", "''")
                            ),
                        }));
                    } else {
                        panic!("No table or view found for {}.{}", schema, table_name);
                    }
                } else {
                    panic!("Expected List node for column comment, found {:?}", list);
                }
            }
            ObjectType::ObjectFunction => {
                let list = &c.object.clone()
                    .expect("Missing object in function comment")
                    .node
                    .expect("Missing node in function comment object");

                if let NodeEnum::ObjectWithArgs(obj) = list {
                    let items = extract_names(&obj.objname, "function comment");
                    validate_item_count(&items, 2, "function comment list");

                    let schema = &items[0];
                    let function_name = &items[1];

                    if find_trigger_function(nodes, schema, function_name) {
                        nodes.push(StatementLocation::TriggerFunction(TriggerFunction {
                            name: function_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON FUNCTION \"{}\".\"{}\" IS E'{}';",
                                schema, function_name, c.comment.replace("'", "''")
                            ),
                        }));
                    } else if find_function(nodes, schema, function_name) {
                        nodes.push(StatementLocation::Function(Function {
                            name: function_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON FUNCTION \"{}\".\"{}\" IS E'{}';",
                                schema, function_name, c.comment.replace("'", "''")
                            ),
                        }));
                    } else {
                        panic!("No trigger or function found for {}.{}", schema, function_name);
                    }
                } else {
                    panic!("Expected ObjectWithArgs for function comment, found {:?}", list);
                }
            }
            pg_query::protobuf::ObjectType::ObjectSchema => {
                let schema_name = get_sval(&c.object.clone()
                    .expect("Missing object in schema comment")
                    .node);

                nodes.push(StatementLocation::Schema(Schema {
                    name: schema_name.to_string(),
                    sql: format!("COMMENT ON SCHEMA \"{}\" IS E'{}';", schema_name, c.comment.replace("'", "''")),
                }));
            }
            ObjectType::ObjectType => {
                let type_node = &c.object.clone()
                    .expect("Missing object in type comment")
                    .node
                    .expect("Missing node in type comment object");

                if let NodeEnum::TypeName(obj) = type_node {
                    let items = extract_names(&obj.names, "type comment");
                    let (schema, type_name) = extract_schema_and_name(&items, "type comment");

                    if find_enum(nodes, schema, type_name) {
                        nodes.push(StatementLocation::EnumNode(Enum {
                            name: type_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON TYPE \"{}\".\"{}\" IS E'{}';",
                                schema, type_name, c.comment.replace("'", "''")
                            ),
                        }));
                    } else if find_composite_type(nodes, schema, type_name) {
                        nodes.push(StatementLocation::CompositeType(CompositeType {
                            name: type_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON TYPE \"{}\".\"{}\" IS E'{}';",
                                schema, type_name, c.comment.replace("'", "''")
                            ),
                        }));
                    } else {
                        panic!("No type found for comment on {}.{}", schema, type_name);
                    }
                } else {
                    panic!("Expected TypeName for type comment, found {:?}", type_node);
                }
            }
            ObjectType::ObjectTable => {
                let list = &c.object.clone()
                    .expect("Missing object in table comment")
                    .node
                    .expect("Missing node in table comment object");

                if let NodeEnum::List(l) = list {
                    let items = extract_names(&l.items, "table comment");
                    let (schema, table_name) = extract_schema_and_name(&items, "table comment");

                    if find_table(nodes, schema, table_name) {
                        nodes.push(StatementLocation::Table(Table {
                            name: table_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON TABLE \"{}\".\"{}\" IS '{}';",
                                schema, table_name, c.comment
                            ),
                        }));
                    } else if find_view(nodes, schema, table_name) {
                        nodes.push(StatementLocation::View(View {
                            name: table_name.to_string(),
                            schema: schema.to_string(),
                            sql: format!(
                                "COMMENT ON VIEW \"{}\".\"{}\" IS '{}';",
                                schema, table_name, c.comment
                            ),
                        }));
                    } else {
                        panic!("No table or view found for {}.{}", schema, table_name);
                    }
                } else {
                    panic!("Expected List for table comment, found {:?}", list);
                }
            }
            _ => {
                panic!("Unsupported comment type: {:?}", c.objtype());
            }
        },
        NodeEnum::CreateEnumStmt(n) => {
            let names = extract_names(&n.type_name, "enum type definition");
            let schema = get_schema_or_default(&names);
            let type_name = names.last()
                .expect("Missing type name in CreateEnumStmt")
                .to_string();

            nodes.push(StatementLocation::EnumNode(Enum {
                schema: schema.to_string(),
                name: type_name,
                sql: sql.to_string(),
            }));
        }
        NodeEnum::DefineStmt(n) => match n.kind() {
            ObjectType::ObjectAggregate => {
                let names = extract_names(&n.defnames, "aggregate definition");
                let schema = get_schema_or_default(&names);
                let type_name = names.last()
                    .expect("Missing aggregate name in definition")
                    .to_string();

                nodes.push(StatementLocation::Aggregate(Aggregate {
                    schema: schema.to_string(),
                    name: type_name,
                    sql: sql.to_string(),
                }));
            }
            ObjectType::ObjectOperator => {
                let names = extract_names(&n.defnames, "operator definition");
                let schema = get_schema_or_default(&names);
                let op_name = names.last()
                    .expect("Missing operator name in definition")
                    .to_string();

                nodes.push(StatementLocation::Operator(Operator {
                    schema: schema.to_string(),
                    name: op_name,
                    sql: sql.to_string(),
                }));
            }
            _ => panic!("Unsupported define statement kind: {:?}", n.kind()),
        },
        pg_query::NodeEnum::CompositeTypeStmt(n) => {
            let name = n.typevar.expect("Missing typevar in CompositeTypeStmt");

            let schema = name.schemaname;
            let type_name = name.relname;

            nodes.push(StatementLocation::CompositeType(CompositeType {
                schema: schema.to_string(),
                name: type_name,
                sql: sql.to_string(),
            }));
        }
        pg_query::NodeEnum::ViewStmt(n) => {
            let rel = n.view.expect("Missing relation in ViewStmt");
            let schema = rel.schemaname;
            let view_name = rel.relname;

            nodes.push(StatementLocation::View(View {
                schema: schema.clone(),
                name: view_name,
                sql: sql.to_string(),
            }));
        }
        pg_query::NodeEnum::CreatePolicyStmt(n) => {
            let name = n.policy_name;
            let table = n.table.expect("Missing table in CreatePolicyStmt");

            let schema = table.schemaname;
            let relation_name = table.relname;

            nodes.push(StatementLocation::Policy(Policy {
                schema: schema.clone(),
                name,
                table: relation_name,
                sql: sql.to_string(),
            }));
        }
        NodeEnum::CreateStmt(n) => {
            let rel = n.relation.expect("Missing relation in CreateStmt");
            let schema = rel.schemaname.clone();
            let table_name = rel.relname.clone();

            nodes.push(StatementLocation::Table(Table {
                schema,
                name: table_name,
                sql: sql.to_string(),
            }));
        }
        NodeEnum::CreateTrigStmt(n) => {
            let rel = n.relation.expect("Missing relation in CreateTrigStmt");
            let schema = rel.schemaname.clone();
            let table_name = rel.relname.clone();

            let func_names = extract_names(&n.funcname, "trigger function");
            let function_name = func_names.last()
                .expect("Missing function name in trigger")
                .to_string();
            let trigger_name = n.trigname.clone();

            nodes.push(StatementLocation::Trigger(Trigger {
                schema,
                name: trigger_name,
                table: table_name,
                function: function_name,
                sql: sql.to_string(),
            }));
        }
        NodeEnum::CreateFunctionStmt(n) => {
            let func_names = extract_names(&n.funcname, "function definition");
            let schema = get_schema_or_default(&func_names).to_string();
            let function_name = func_names.last()
                .expect("Missing function name")
                .to_string();

            let return_type = n.return_type.as_ref()
                .expect("Missing return type in function");

            let is_trigger = return_type.names.iter().any(|n| {
                let type_name = get_sval(&n.node);
                type_name == "trigger"
            });

            if is_trigger {
                nodes.push(StatementLocation::TriggerFunction(TriggerFunction {
                    schema,
                    name: function_name,
                    sql: sql.to_string(),
                }));
            } else {
                nodes.push(StatementLocation::Function(Function {
                    schema,
                    name: function_name,
                    sql: sql.to_string(),
                }));
            }
        }
        pg_query::NodeEnum::IndexStmt(n) => {
            let rel = n.relation.expect("Missing relation in IndexStmt");
            let schema = rel.schemaname;
            let index_name = n.idxname;
            let table_name = rel.relname;

            nodes.push(StatementLocation::Index(Index {
                schema: schema.clone(),
                name: index_name,
                table: table_name,
                sql: sql.to_string(),
            }));
        }
        pg_query::NodeEnum::AlterTableStmt(n) => {
            let rel = n.relation.expect("Missing relation in AlterTableStmt");
            let schema = rel.schemaname;
            let table_name = rel.relname;

            let number_of_commands = n.cmds.len();
            if number_of_commands == 0 {
                panic!("No commands in AlterTableStmt");
            }

            let cmd = n.cmds.first()
                .expect("Missing command in AlterTableStmt")
                .node.clone()
                .expect("Missing node in AlterTableStmt command");

            match &cmd {
                pg_query::NodeEnum::AlterTableCmd(c) => match c.subtype() {
                    pg_query::protobuf::AlterTableType::AtColumnDefault => {
                        nodes.push(StatementLocation::Table(Table {
                            schema: schema.clone(),
                            name: table_name.clone(),
                            sql: sql.to_string(),
                        }));
                    }
                    pg_query::protobuf::AlterTableType::AtEnableRowSecurity => {
                        nodes.push(StatementLocation::EnablePolicy(EnablePolicy {
                            schema,
                            table: table_name,
                            sql: sql.to_string(),
                        }));
                    }
                    pg_query::protobuf::AlterTableType::AtAddConstraint => {
                        if number_of_commands > 1 {
                            let add_constraint_idx = sql.find("ADD CONSTRAINT")
                                .expect("Expected 'ADD CONSTRAINT' in SQL");

                            let commands = sql[add_constraint_idx..]
                                .split("ADD CONSTRAINT")
                                .collect::<Vec<_>>();

                            // get from beginning to first ADD CONSTRAINT
                            let begin = sql[sql.find("ALTER TABLE")
                                .expect("Expected 'ALTER TABLE' in SQL")
                                ..add_constraint_idx]
                                .to_string();

                            commands.iter().for_each(|cmd| {
                                if cmd.is_empty() {
                                    return;
                                }

                                let full_sql = format!(
                                    "{}ADD CONSTRAINT{}",
                                    begin,
                                    cmd.trim_end().trim_end_matches(',')
                                );
                                parse(&full_sql, nodes);
                            });
                        } else if let Some(pg_query::protobuf::node::Node::Constraint(c)) =
                            c.def.clone()
                            .expect("Missing constraint definition")
                            .node.as_ref()
                        {
                            match c.contype() {
                                pg_query::protobuf::ConstrType::ConstrForeign => {
                                    let constraint_name = c.conname.clone();
                                    let source_schema = schema.clone();
                                    let source_table = table_name.clone();
                                    let pktable = c
                                        .pktable
                                        .as_ref()
                                        .expect("Missing target table for foreign key");
                                    let target_schema = pktable.schemaname.clone();
                                    let target_table = pktable.relname.clone();

                                    nodes.push(StatementLocation::ForeignKey(ForeignKey {
                                        constraint_name,
                                        source_schema,
                                        source_table,
                                        target_schema,
                                        target_table,
                                        sql: sql.to_string(),
                                    }));
                                }
                                pg_query::protobuf::ConstrType::ConstrPrimary
                                | pg_query::protobuf::ConstrType::ConstrUnique
                                | pg_query::protobuf::ConstrType::ConstrCheck
                                | pg_query::protobuf::ConstrType::ConstrExclusion => {
                                    let source_schema = schema.clone();
                                    let source_table = table_name.clone();

                                    nodes.push(StatementLocation::Table(Table {
                                        name: source_table,
                                        schema: source_schema,
                                        sql: sql.to_string(),
                                    }));
                                }
                                _ => {
                                    panic!("Unsupported constraint type: {:?}", c.contype());
                                }
                            }
                        } else {
                            panic!("Missing definition for constraint");
                        }
                    }
                    pg_query::protobuf::AlterTableType::AtChangeOwner => {} // Skip ownership changes
                    _ => panic!("Unsupported AlterTableType: {:?} for SQL: '{}'", c.subtype(), sql),
                },
                _ => panic!("Unsupported command in AlterTableStmt: {:?} for SQL: '{}'", cmd, sql),
            }
        }
        pg_query::NodeEnum::VariableSetStmt(n) => {
            if n.kind() != pg_query::protobuf::VariableSetKind::VarResetAll {
                nodes.push(StatementLocation::Setup(Setup {
                    sql: sql.to_string(),
                }));
            }
        }
        pg_query::NodeEnum::SelectStmt(_) => {
            nodes.push(StatementLocation::Setup(Setup {
                sql: sql.to_string(),
            }));
        }
        pg_query::NodeEnum::AlterOwnerStmt(n) => match n.object_type() {
            pg_query::protobuf::ObjectType::ObjectSchema => {
                let schema_name = get_sval(&n.object
                    .expect("Missing object in AlterOwnerStmt")
                    .node);

                nodes.push(StatementLocation::Schema(Schema {
                    name: schema_name,
                    sql: sql.to_string(),
                }));
            }
            pg_query::protobuf::ObjectType::ObjectAggregate => {
                let list = &n.object.clone()
                    .expect("Missing object in AlterOwnerStmt")
                    .node
                    .expect("Missing node in AlterOwnerStmt object");

                if let pg_query::NodeEnum::ObjectWithArgs(obj) = list {
                    let items = obj
                        .objname
                        .iter()
                        .map(|n| get_sval(&n.node))
                        .collect::<Vec<_>>();

                    if items.len() != 2 {
                        panic!("Expected 2 items in aggregate owner list, found {}", items.len());
                    }

                    let schema = items.first()
                        .expect("Missing schema in aggregate owner");
                    let agg_name = items.last()
                        .expect("Missing aggregate name in owner");

                    nodes.push(StatementLocation::Aggregate(Aggregate {
                        name: agg_name.to_string(),
                        schema: schema.to_string(),
                        sql: sql.to_string(),
                    }));
                } else {
                    panic!("Expected ObjectWithArgs for aggregate owner, found {:?}", list);
                }
            }
            pg_query::protobuf::ObjectType::ObjectOperator => {
                let list = &n.object.clone()
                    .expect("Missing object in AlterOwnerStmt")
                    .node
                    .expect("Missing node in AlterOwnerStmt object");

                if let pg_query::NodeEnum::ObjectWithArgs(obj) = list {
                    let items = obj
                        .objname
                        .iter()
                        .map(|n| get_sval(&n.node))
                        .collect::<Vec<_>>();

                    if items.len() != 2 {
                        panic!("Expected 2 items in operator owner list, found {}", items.len());
                    }

                    let schema = items.first()
                        .expect("Missing schema in operator owner");
                    let op_name = items.last()
                        .expect("Missing operator name in owner");

                    nodes.push(StatementLocation::Operator(Operator {
                        name: op_name.to_string(),
                        schema: schema.to_string(),
                        sql: sql.to_string(),
                    }));
                } else {
                    panic!("Expected ObjectWithArgs for operator owner, found {:?}", list);
                }
            }
            pg_query::protobuf::ObjectType::ObjectFunction => {
                let list = &n.object.clone()
                    .expect("Missing object in AlterOwnerStmt")
                    .node
                    .expect("Missing node in AlterOwnerStmt object");

                if let pg_query::NodeEnum::ObjectWithArgs(obj) = list {
                    let items = obj
                        .objname
                        .iter()
                        .map(|n| get_sval(&n.node))
                        .collect::<Vec<_>>();

                    if items.len() != 2 {
                        panic!("Expected 2 items in function owner list, found {}", items.len());
                    }

                    let schema = items.first()
                        .expect("Missing schema in function owner");
                    let function_name = items.last()
                        .expect("Missing function name in owner");

                    if find_trigger_function(nodes, schema, function_name) {
                        nodes.push(StatementLocation::TriggerFunction(TriggerFunction {
                            name: function_name.to_string(),
                            schema: schema.to_string(),
                            sql: sql.to_string(),
                        }));
                    } else {
                        nodes.push(StatementLocation::Function(Function {
                            name: function_name.to_string(),
                            schema: schema.to_string(),
                            sql: sql.to_string(),
                        }));
                    }
                } else {
                    panic!("Expected ObjectWithArgs for function owner, found {:?}", list);
                }
            }
            pg_query::protobuf::ObjectType::ObjectType => {
                if let pg_query::NodeEnum::List(l) = n.object
                    .expect("Missing object in AlterOwnerStmt")
                    .node
                    .expect("Missing node in AlterOwnerStmt object")
                {
                    let items = l
                        .items
                        .iter()
                        .map(|n| get_sval(&n.node))
                        .collect::<Vec<_>>();

                    if items.len() != 2 {
                        panic!("Expected 2 items in type owner list, found {}", items.len());
                    }

                    let schema = items.first()
                        .expect("Missing schema in type owner");
                    let type_name = items.get(1)
                        .expect("Missing type name in owner");

                    if nodes.iter().any(|n| {
                        if let StatementLocation::EnumNode(e) = n {
                            e.name == *type_name && e.schema == *schema
                        } else {
                            false
                        }
                    }) {
                        nodes.push(StatementLocation::EnumNode(Enum {
                            name: type_name.to_string(),
                            schema: schema.to_string(),
                            sql: sql.to_string(),
                        }));
                    } else if nodes.iter().any(|n| {
                        if let StatementLocation::CompositeType(t) = n {
                            t.name == *type_name && t.schema == *schema
                        } else {
                            false
                        }
                    }) {
                        nodes.push(StatementLocation::CompositeType(CompositeType {
                            name: type_name.to_string(),
                            schema: schema.to_string(),
                            sql: sql.to_string(),
                        }));
                    } else {
                        panic!(
                            "No enum or composite type found for {}.{}",
                            schema, type_name
                        );
                    }
                } else {
                    panic!("Expected List for type owner");
                }
            }
            _ => {
                panic!("Unsupported object type in AlterOwnerStmt: {:?}", n.object_type());
            }
        },
        pg_query::NodeEnum::CreateSeqStmt(n) => {
            let range_var = n.sequence.expect("Missing sequence in CreateSeqStmt");
            let schema_name = range_var.schemaname;
            let rel_name = range_var.relname;

            nodes.push(StatementLocation::Sequence(Sequence {
                table: None,
                name: rel_name.to_string(),
                schema: schema_name.to_string(),
                sql: sql.to_string(),
            }));
        }
        pg_query::NodeEnum::AlterSeqStmt(n) => {
            let range_var = n.sequence.expect("Missing sequence in AlterSeqStmt");
            let schema_name = range_var.schemaname;
            let rel_name = range_var.relname;

            let opts = n
                .options
                .iter()
                .find_map(|o| {
                    if let pg_query::NodeEnum::DefElem(d) = &o.node
                        .clone()
                        .expect("Missing node in AlterSeqStmt option")
                    {
                        if d.defname == "owned_by" {
                            Some(d.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .expect("Only owned_by is supported in AlterSeqStmt");

            if let pg_query::NodeEnum::List(l) = opts.arg
                .expect("Missing arg in owned_by option")
                .node
                .expect("Missing node in owned_by option")
            {
                let items = l
                    .items
                    .iter()
                    .map(|n| get_sval(&n.node))
                    .collect::<Vec<_>>();

                if items.len() != 3 {
                    panic!("Expected 3 items in sequence owned_by list, found {}", items.len());
                }

                let schema = items.first()
                    .expect("Missing schema in sequence owned_by");
                if *schema != schema_name {
                    panic!("Schema name mismatch in sequence owned_by: {} != {}", schema, schema_name);
                }
                let table_name = items.get(1)
                    .expect("Missing table name in sequence owned_by");

                nodes.push(StatementLocation::Sequence(Sequence {
                    table: Some(table_name.clone()),
                    name: rel_name.to_string(),
                    schema: schema_name.to_string(),
                    sql: sql.to_string(),
                }));
            } else {
                panic!("Expected List for sequence owned_by");
            }
        }
        pg_query::NodeEnum::GrantStmt(n) => {
            match n.objtype() {
                pg_query::protobuf::ObjectType::ObjectSchema => {
                    let schema_name = get_sval(&n.objects.first()
                        .expect("Missing object in GrantStmt")
                        .node);

                    nodes.push(StatementLocation::Schema(Schema {
                        name: schema_name.to_string(),
                        sql: sql.to_string(),
                    }));
                }
                pg_query::protobuf::ObjectType::ObjectTable => {
                    let range_var = &n.objects.first()
                        .expect("Missing object in table grant")
                        .node
                        .clone()
                        .expect("Missing node in table grant object");

                    if let pg_query::NodeEnum::RangeVar(obj) = range_var {
                        let schema = obj.schemaname.clone();
                        let name = obj.relname.clone();

                        if find_table(nodes, &schema, &name) {
                            nodes.push(StatementLocation::Table(Table {
                                schema,
                                name,
                                sql: sql.to_string(),
                            }));
                        } else if find_view(nodes, &schema, &name) {
                            nodes.push(StatementLocation::View(View {
                                schema,
                                name,
                                sql: sql.to_string(),
                            }));
                        } else {
                            panic!("No table or view found for {}.{}", schema, name);
                        }
                    } else {
                        panic!("Expected RangeVar for table grant, found {:?}", range_var);
                    }
                }
                pg_query::protobuf::ObjectType::ObjectSequence => {
                    let range_var = &n.objects.first()
                        .expect("Missing object in sequence grant")
                        .node
                        .clone()
                        .expect("Missing node in sequence grant object");

                    if let pg_query::NodeEnum::RangeVar(obj) = range_var {
                        nodes.push(StatementLocation::Sequence(Sequence {
                            table: None,
                            schema: obj.schemaname.clone(),
                            name: obj.relname.clone(),
                            sql: sql.to_string(),
                        }));
                    } else {
                        panic!("Expected RangeVar for sequence grant, found {:?}", range_var);
                    }
                }
                pg_query::protobuf::ObjectType::ObjectFunction => {
                    let list = &n.objects.first()
                        .expect("Missing object in function grant")
                        .node
                        .clone()
                        .expect("Missing node in function grant object");

                    if let pg_query::NodeEnum::ObjectWithArgs(obj) = list {
                        let items = obj
                            .objname
                            .iter()
                            .map(|n| get_sval(&n.node))
                            .collect::<Vec<_>>();

                        if items.len() != 2 {
                            panic!("Expected 2 items in function grant list, found {}", items.len());
                        }

                        let schema = items.first()
                            .expect("Missing schema in function grant");
                        let function_name = items.last()
                            .expect("Missing function name in function grant");

                        if find_trigger_function(nodes, schema, function_name) {
                            nodes.push(StatementLocation::TriggerFunction(TriggerFunction {
                                name: function_name.to_string(),
                                schema: schema.to_string(),
                                sql: sql.to_string(),
                            }));
                        } else if find_function(nodes, schema, function_name) {
                            nodes.push(StatementLocation::Function(Function {
                                name: function_name.to_string(),
                                schema: schema.to_string(),
                                sql: sql.to_string(),
                            }));
                        } else if find_aggregate(nodes, schema, function_name) {
                            nodes.push(StatementLocation::Aggregate(Aggregate {
                                name: function_name.to_string(),
                                schema: schema.to_string(),
                                sql: sql.to_string(),
                            }));
                        } else {
                            panic!("No trigger or function or aggregate found for {}.{}", schema, function_name);
                        }
                    } else {
                        panic!("Expected ObjectWithArgs for function grant, found {:?}", list);
                    }
                }
                _ => {
                    panic!("Unsupported object type in GrantStmt: {:?}", n.objtype());
                }
            };
        }
        pg_query::NodeEnum::AlterDefaultPrivilegesStmt(n) => {
            // Extract schema from options
            let schema_elem = n
                .options
                .iter()
                .find_map(|o| {
                    if let pg_query::NodeEnum::DefElem(d) = &o.node
                        .clone()
                        .expect("Missing node in AlterDefaultPrivilegesStmt option")
                    {
                        if d.defname == "schemas" {
                            Some(d.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .expect("schemas option is required in AlterDefaultPrivilegesStmt");

            // Extract schema name from the list
            let schema_name = if let pg_query::NodeEnum::List(l) =
                schema_elem.arg
                    .expect("Missing arg in schema option")
                    .node
                    .expect("Missing node in schema option arg")
            {
                if let Some(item) = l.items.first() {
                    if let pg_query::NodeEnum::String(s) = &item.node
                        .clone()
                        .expect("Missing node in schema list item")
                    {
                        s.sval.clone()
                    } else {
                        panic!("Expected String in schema list, found {:?}", item.node);
                    }
                } else {
                    panic!("Empty schema list in AlterDefaultPrivilegesStmt");
                }
            } else {
                panic!("Expected List for schemas in AlterDefaultPrivilegesStmt");
            };

            nodes.push(StatementLocation::Schema(Schema {
                name: schema_name,
                sql: sql.to_string(),
            }));
        }
        _ => panic!("Unsupported node:\n{:?} '{}'", node, sql),
    };
}

pub fn get_sval(n: &Option<pg_query::protobuf::node::Node>) -> String {
    match n {
        Some(pg_query::protobuf::node::Node::String(s)) => s.sval.clone(),
        _ => panic!("Expected String node, found {:?}", n),
    }
}

fn parse_sql(sql: &str) -> pg_query::NodeEnum {
    pg_query::parse(sql)
        .expect("Failed to parse SQL")
        .protobuf
        .nodes()
        .iter()
        .find(|n| n.1 == 1)
        .map(|n| n.0.to_enum())
        .expect("Failed to find root node in parsed SQL")
}

/// Extract a list of strings from names in a node
fn extract_names(items: &[Node], _context: &str) -> Vec<String> {
    items
        .iter()
        .map(|n| get_sval(&n.node))
        .collect::<Vec<_>>()
}

/// Helper to check if a name exists in nodes of a specific type
fn find_node_by_name<F>(nodes: &[StatementLocation], schema: &str, name: &str, matcher: F) -> bool
where
    F: Fn(&StatementLocation) -> Option<(&String, &String)>,
{
    nodes.iter().any(|node| {
        if let Some((node_schema, node_name)) = matcher(node) {
            node_name == name && node_schema == schema
        } else {
            false
        }
    })
}

/// Helper to check if a name exists in nodes of a specific type
fn nodes_by_name<'a, F>(nodes: &'a[StatementLocation], schema: &'a str, name: &'a str, matcher: F) -> Vec<&'a StatementLocation>
where
    F: Fn(&StatementLocation) -> Option<(&String, &String)>,
{
    nodes.iter().filter(|node| {
        if let Some((node_schema, node_name)) = matcher(node) {
            node_name == name && node_schema == schema
        } else {
            false
        }
    }).collect()
}

/// Validate that a list of items has exactly the expected count
fn validate_item_count(items: &[String], expected: usize, context: &str) {
    if items.len() != expected {
        panic!("Expected {} items in {}, found {}", expected, context, items.len());
    }
}

/// Extract schema and name from a qualified name list
fn extract_schema_and_name<'a>(items: &'a [String], context: &str) -> (&'a str, &'a str) {
    validate_item_count(items, 2, context);
    let schema = &items[0];
    let name = &items[1];
    (schema, name)
}

/// Helper to get schema from a name list, defaults to "public" if only one item
fn get_schema_or_default(names: &[String]) -> &str {
    if names.len() > 1 {
        &names[0]
    } else {
        "public"
    }
}

/// Check if a table with given schema and name exists
fn find_table(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::Table(t) = node {
            Some((&t.schema, &t.name))
        } else {
            None
        }
    })
}

/// Check if a view with given schema and name exists
fn find_view(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::View(v) = node {
            Some((&v.schema, &v.name))
        } else {
            None
        }
    })
}

/// Check if an enum type with given schema and name exists
fn find_enum(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::EnumNode(e) = node {
            Some((&e.schema, &e.name))
        } else {
            None
        }
    })
}

/// Check if a composite type with given schema and name exists
fn find_composite_type(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::CompositeType(t) = node {
            Some((&t.schema, &t.name))
        } else {
            None
        }
    })
}

/// Check if a trigger function with given schema and name exists
fn find_trigger_function(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::TriggerFunction(t) = node {
            Some((&t.schema, &t.name))
        } else {
            None
        }
    })
}

/// Check if a function with given schema and name exists
fn find_function(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::Function(t) = node {
            Some((&t.schema, &t.name))
        } else {
            None
        }
    })
}

/// Check if a function with given schema and name exists
fn find_aggregate(nodes: &[StatementLocation], schema: &str, name: &str) -> bool {
    find_node_by_name(nodes, schema, name, |node| {
        if let StatementLocation::Aggregate(t) = node {
            Some((&t.schema, &t.name))
        } else {
            None
        }
    })
}
