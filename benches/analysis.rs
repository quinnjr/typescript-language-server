use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tree_sitter::Parser;

// We need to make the analysis module accessible from benches
// For now, we'll inline a simplified version of the binder

/// Generate TypeScript code with varying complexity
fn generate_code_with_scopes(depth: usize, siblings: usize) -> String {
    let mut code = String::new();

    code.push_str("// Generated benchmark code\n\n");

    fn generate_nested(code: &mut String, depth: usize, siblings: usize, prefix: &str) {
        if depth == 0 {
            return;
        }

        for i in 0..siblings {
            let name = format!("{}_{}", prefix, i);

            code.push_str(&format!(
                r#"
function {name}(param{i}: string): number {{
    const local{i} = param{i}.length;
    let mutable{i} = local{i} * 2;

    if (mutable{i} > 10) {{
        const nested{i} = mutable{i} + 1;
        mutable{i} = nested{i};
    }}

"#
            ));

            generate_nested(code, depth - 1, siblings.saturating_sub(1), &name);

            code.push_str(&format!(
                r#"
    return mutable{i};
}}
"#
            ));
        }
    }

    generate_nested(&mut code, depth, siblings, "func");
    code
}

/// Count all identifiers in a tree
fn count_identifiers(tree: &tree_sitter::Tree) -> usize {
    let mut count = 0;
    let mut cursor = tree.walk();

    loop {
        if cursor.node().kind() == "identifier" {
            count += 1;
        }

        if cursor.goto_first_child() {
            continue;
        }

        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return count;
            }
        }
    }
}

/// Extract all declarations from a tree
fn extract_declarations(tree: &tree_sitter::Tree, source: &str) -> Vec<(String, String)> {
    let mut declarations = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();
        let kind = node.kind();

        match kind {
            "function_declaration" | "class_declaration" | "interface_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        declarations.push((kind.to_string(), name.to_string()));
                    }
                }
            }
            "variable_declarator" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if name_node.kind() == "identifier" {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            declarations.push(("variable".to_string(), name.to_string()));
                        }
                    }
                }
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            continue;
        }

        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return declarations;
            }
        }
    }
}

fn bench_symbol_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Symbol Extraction");

    for (depth, siblings) in [(3, 3), (4, 4), (5, 3)].iter() {
        let code = generate_code_with_scopes(*depth, *siblings);

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(&code, None).unwrap();

        group.bench_with_input(
            BenchmarkId::new("declarations", format!("depth={},siblings={}", depth, siblings)),
            &(&tree, &code),
            |b, (tree, source)| {
                b.iter(|| {
                    let decls = extract_declarations(black_box(tree), black_box(source));
                    black_box(decls)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("identifiers", format!("depth={},siblings={}", depth, siblings)),
            &tree,
            |b, tree| {
                b.iter(|| {
                    let count = count_identifiers(black_box(tree));
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

fn bench_scope_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scope Building");

    let code = generate_code_with_scopes(4, 4);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let tree = parser.parse(&code, None).unwrap();

    // Simulate scope building by traversing and tracking depth
    group.bench_function("track_scope_depth", |b| {
        b.iter(|| {
            let mut max_depth: usize = 0;
            let mut current_depth: usize = 0;
            let mut cursor = tree.walk();

            loop {
                let node = cursor.node();

                // Track scope-creating nodes
                match node.kind() {
                    "function_declaration" | "arrow_function" | "method_definition"
                    | "class_declaration" | "statement_block" | "if_statement"
                    | "for_statement" | "while_statement" => {
                        current_depth += 1;
                        max_depth = max_depth.max(current_depth);
                    }
                    _ => {}
                }

                if cursor.goto_first_child() {
                    continue;
                }

                while !cursor.goto_next_sibling() {
                    // Leaving scope
                    let leaving = cursor.node();
                    match leaving.kind() {
                        "function_declaration" | "arrow_function" | "method_definition"
                        | "class_declaration" | "statement_block" | "if_statement"
                        | "for_statement" | "while_statement" => {
                            current_depth = current_depth.saturating_sub(1);
                        }
                        _ => {}
                    }

                    if !cursor.goto_parent() {
                        return black_box(max_depth);
                    }
                }
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_symbol_extraction, bench_scope_building);
criterion_main!(benches);

