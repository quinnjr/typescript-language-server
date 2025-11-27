use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tree_sitter::Parser;

/// Sample TypeScript code of various sizes
fn generate_typescript_code(num_functions: usize) -> String {
    let mut code = String::new();

    // Imports
    code.push_str("import { useState, useEffect, useCallback } from 'react';\n");
    code.push_str("import type { FC, ReactNode } from 'react';\n\n");

    // Interfaces
    code.push_str("interface User {\n");
    code.push_str("  id: number;\n");
    code.push_str("  name: string;\n");
    code.push_str("  email: string;\n");
    code.push_str("  createdAt: Date;\n");
    code.push_str("}\n\n");

    code.push_str("interface Props {\n");
    code.push_str("  users: User[];\n");
    code.push_str("  onSelect: (user: User) => void;\n");
    code.push_str("}\n\n");

    // Generate functions
    for i in 0..num_functions {
        code.push_str(&format!(
            r#"
/**
 * Process user data - function {i}
 * @param user The user to process
 * @returns The processed result
 */
export function processUser{i}(user: User): {{ id: number; displayName: string }} {{
    const {{ id, name, email }} = user;
    const displayName = `${{name}} <${{email}}>`;

    if (id < 0) {{
        throw new Error('Invalid user ID');
    }}

    const result = {{
        id,
        displayName,
    }};

    return result;
}}

export const fetchUser{i} = async (id: number): Promise<User | null> => {{
    try {{
        const response = await fetch(`/api/users/${{id}}`);
        if (!response.ok) {{
            return null;
        }}
        const data = await response.json();
        return data as User;
    }} catch (error) {{
        console.error('Failed to fetch user:', error);
        return null;
    }}
}};

export class UserService{i} {{
    private cache: Map<number, User> = new Map();

    constructor(private baseUrl: string) {{}}

    async getUser(id: number): Promise<User | null> {{
        if (this.cache.has(id)) {{
            return this.cache.get(id) ?? null;
        }}

        const user = await fetchUser{i}(id);
        if (user) {{
            this.cache.set(id, user);
        }}
        return user;
    }}

    clearCache(): void {{
        this.cache.clear();
    }}
}}
"#
        ));
    }

    code
}

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("TypeScript Parsing");

    // Test different file sizes
    for num_functions in [10, 50, 100, 200].iter() {
        let code = generate_typescript_code(*num_functions);
        let bytes = code.len();

        group.throughput(Throughput::Bytes(bytes as u64));

        group.bench_with_input(
            BenchmarkId::new("tree-sitter", format!("{} functions", num_functions)),
            &code,
            |b, code| {
                let mut parser = Parser::new();
                parser
                    .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .unwrap();

                b.iter(|| {
                    let tree = parser.parse(black_box(code), None);
                    black_box(tree)
                });
            },
        );
    }

    group.finish();
}

fn bench_incremental_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Incremental Parsing");

    let code = generate_typescript_code(50);
    let bytes = code.len();

    group.throughput(Throughput::Bytes(bytes as u64));

    // First parse
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let initial_tree = parser.parse(&code, None).unwrap();

    // Create a modified version (simulating a small edit)
    let mut modified_code = code.clone();
    let edit_point = modified_code.find("displayName").unwrap_or(100);
    modified_code.insert_str(edit_point, "/* comment */");

    group.bench_function("with_old_tree", |b| {
        b.iter(|| {
            let tree = parser.parse(black_box(&modified_code), Some(&initial_tree));
            black_box(tree)
        });
    });

    group.bench_function("without_old_tree", |b| {
        b.iter(|| {
            let tree = parser.parse(black_box(&modified_code), None);
            black_box(tree)
        });
    });

    group.finish();
}

fn bench_tree_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tree Traversal");

    let code = generate_typescript_code(100);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let tree = parser.parse(&code, None).unwrap();

    group.bench_function("count_nodes", |b| {
        b.iter(|| {
            let mut count = 0;
            let mut cursor = tree.walk();

            loop {
                count += 1;

                if cursor.goto_first_child() {
                    continue;
                }

                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        return black_box(count);
                    }
                }
            }
        });
    });

    group.bench_function("find_functions", |b| {
        b.iter(|| {
            let mut functions = Vec::new();
            let mut cursor = tree.walk();

            loop {
                let node = cursor.node();
                if node.kind() == "function_declaration"
                    || node.kind() == "arrow_function"
                    || node.kind() == "method_definition"
                {
                    functions.push(node);
                }

                if cursor.goto_first_child() {
                    continue;
                }

                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        return black_box(functions);
                    }
                }
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_incremental_parsing,
    bench_tree_traversal
);
criterion_main!(benches);

