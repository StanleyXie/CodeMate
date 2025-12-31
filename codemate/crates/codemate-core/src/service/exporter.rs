use crate::service::models::ModuleResponse;
use serde_json::json;

pub struct ModuleGraphExporter;

impl ModuleGraphExporter {
    pub fn to_dot(modules: &[ModuleResponse]) -> String {
        let mut dot = String::from("digraph ModuleGraph {\n");
        dot.push_str("  node [shape=box, fontname=\"Arial\"];\n");
        dot.push_str("  rankdir=LR;\n\n");

        let mut has_detailed_edges = false;
        for m in modules {
            if m.dependencies.iter().any(|d| d.edges.is_some()) {
                has_detailed_edges = true;
                break;
            }
        }

        if !has_detailed_edges {
            // Summary view (current behavior)
            for m_resp in modules {
                let label = format!("{} ({})", m_resp.module.name, m_resp.module.project_type.as_str());
                dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", m_resp.module.id, label));

                for dep in &m_resp.dependencies {
                    dot.push_str(&format!(
                        "  \"{}\" -> \"{}\" [label=\"{} edges\"];\n",
                        m_resp.module.id, dep.target_id, dep.count
                    ));
                }
            }
        } else {
            // Detailed E2E view
            for m_resp in modules {
                dot.push_str(&format!("  subgraph \"cluster_{}\" {{\n", Self::sanitize_id(&m_resp.module.id)));
                dot.push_str(&format!("    label=\"{} ({})\";\n", m_resp.module.name, m_resp.module.project_type.as_str()));
                dot.push_str("    style=filled; color=lightgrey;\n");

                // Collect all symbols in this module that are sources or targets
                let mut symbols = std::collections::HashSet::new();
                for dep in &m_resp.dependencies {
                    if let Some(ref edges) = dep.edges {
                        for edge in edges {
                            symbols.insert(edge.source_symbol.clone());
                        }
                    }
                }
                
                for sym in symbols {
                    let sym_id = format!("{}_{}", Self::sanitize_id(&m_resp.module.id), Self::sanitize_id(&sym));
                    dot.push_str(&format!("    \"{}\" [label=\"{}\", style=filled, color=white];\n", sym_id, sym));
                }
                dot.push_str("  }\n");
            }

            // Draw edges between symbols
            for m_resp in modules {
                for dep in &m_resp.dependencies {
                    if let Some(ref edges) = dep.edges {
                        for edge in edges {
                            let src_id = format!("{}_{}", Self::sanitize_id(&m_resp.module.id), Self::sanitize_id(&edge.source_symbol));
                            let tgt_id = format!("{}_{}", Self::sanitize_id(&dep.target_id), Self::sanitize_id(&edge.target_symbol));
                            let label = if let Some(line) = edge.line_number {
                                format!("L{}", line)
                            } else {
                                String::new()
                            };
                            dot.push_str(&format!("  \"{}\" -> \"{}\" [label=\"{}\"];\n", src_id, tgt_id, label));
                        }
                    }
                }
            }
        }

        dot.push_str("}\n");
        dot
    }

    pub fn to_mermaid(modules: &[ModuleResponse]) -> String {
        let mut mermaid = String::from("flowchart LR\n");
        
        // 1. Build a group map for nesting
        let mut module_map: std::collections::HashMap<String, &ModuleResponse> = std::collections::HashMap::new();
        let mut children_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for m in modules {
            module_map.insert(m.module.id.clone(), m);
        }

        for m in modules {
            if let Some(ref pid) = m.module.parent_id {
                // Only consider it a child if the parent is also in our display set
                if module_map.contains_key(pid) {
                    children_map.entry(pid.clone()).or_default().push(m.module.id.clone());
                }
            }
        }

        // Roots are modules whose parent ID is either None or NOT in our current set
        let mut root_ids = Vec::new();
        for m in modules {
            match m.module.parent_id {
                None => root_ids.push(m.module.id.clone()),
                Some(ref pid) if !module_map.contains_key(pid) => root_ids.push(m.module.id.clone()),
                _ => {}
            }
        }

        // Check if we have any detailed edges
        let mut has_detailed_edges = false;
        for m in modules {
            if m.dependencies.iter().any(|d| d.edges.is_some()) {
                has_detailed_edges = true;
                break;
            }
        }

        // 1.5. Build incoming edges map for faster symbol lookup
        let mut incoming_map: std::collections::HashMap<String, Vec<&ModuleResponse>> = std::collections::HashMap::new();
        if has_detailed_edges {
            for m in modules {
                for dep in &m.dependencies {
                    incoming_map.entry(dep.target_id.clone()).or_default().push(m);
                }
            }
        }

        // 2. Recursive function to render subgraphs
        fn render_subgraph(
            current_id: &str,
            children_map: &std::collections::HashMap<String, Vec<String>>,
            module_map: &std::collections::HashMap<String, &ModuleResponse>,
            incoming_map: &std::collections::HashMap<String, Vec<&ModuleResponse>>,
            mermaid: &mut String,
            indent: &str,
            has_detailed_edges: bool
        ) {
            let m_resp = match module_map.get(current_id) {
                Some(m) => *m,
                None => return,
            };

            let safe_id = ModuleGraphExporter::sanitize_id(&m_resp.module.id);
            // Prefix subgraph ID to avoid collision with node IDs
            mermaid.push_str(&format!("{}subgraph sg_{}\n", indent, safe_id));
            mermaid.push_str(&format!("{}    direction LR\n", indent));
            
            // Representative node for the module itself
            mermaid.push_str(&format!(
                "{}    node_{}[\"{} ({})\"]\n", 
                indent, 
                safe_id, 
                m_resp.module.name,
                m_resp.module.project_type.as_str()
            ));
            if has_detailed_edges {
                mermaid.push_str(&format!("{}    style node_{} fill:#e1f5fe,stroke:#01579b\n", indent, safe_id));
            }

            // Render nested subgraphs
            if let Some(children) = children_map.get(current_id) {
                for child_id in children {
                    render_subgraph(
                        child_id,
                        children_map,
                        module_map,
                        incoming_map,
                        mermaid,
                        &format!("{}    ", indent),
                        has_detailed_edges
                    );
                }
            }

            // Render symbols in this module if detailed
            if has_detailed_edges {
                let mut symbols = std::collections::HashMap::new();
                // Outgoing symbols
                for dep in &m_resp.dependencies {
                    if let Some(ref edges) = dep.edges {
                        for edge in edges {
                            symbols.insert(edge.source_symbol.clone(), edge.source_kind);
                        }
                    }
                }
                // Incoming symbols
                if let Some(incomers) = incoming_map.get(current_id) {
                    for other_m in incomers {
                        for dep in &other_m.dependencies {
                            if dep.target_id == m_resp.module.id {
                                if let Some(ref edges) = dep.edges {
                                    for edge in edges {
                                        symbols.insert(edge.target_symbol.clone(), edge.target_kind);
                                    }
                                }
                            }
                        }
                    }
                }

                let mut sorted_symbols: Vec<_> = symbols.into_iter().collect();
                sorted_symbols.sort_by(|a, b| a.0.cmp(&b.0));

                for (sym, kind) in sorted_symbols {
                    let sym_id = format!("{}_{}", safe_id, ModuleGraphExporter::sanitize_id(&sym));
                    let label = if let Some(k) = kind {
                        format!("{} ({})", sym, k.as_str())
                    } else {
                        sym.clone()
                    };
                    mermaid.push_str(&format!("{}    {}[\"{}\"]\n", indent, sym_id, label));
                }
            }

            mermaid.push_str(&format!("{}end\n", indent));
        }

        for root_id in root_ids {
            render_subgraph(&root_id, &children_map, &module_map, &incoming_map, &mut mermaid, "    ", has_detailed_edges);
        }

        // 3. Define edges
        // 3a. Add structural aggregation edges (Application -> Module)
        for (pid, children) in &children_map {
            let p_node_id = format!("node_{}", Self::sanitize_id(pid));
            for cid in children {
                let c_node_id = format!("node_{}", Self::sanitize_id(cid));
                mermaid.push_str(&format!("    {} -.->|aggregates| {}\n", p_node_id, c_node_id));
            }
        }

        // 3b. Add dependency edges
        for m_resp in modules {
            let src_safe_id = Self::sanitize_id(&m_resp.module.id);
            for dep in &m_resp.dependencies {
                let tgt_safe_id = Self::sanitize_id(&dep.target_id);
                
                if let Some(ref edges) = dep.edges {
                    // Detailed symbol-to-symbol edges
                    for edge in edges {
                        let src_id = format!("{}_{}", src_safe_id, Self::sanitize_id(&edge.source_symbol));
                        let tgt_id = format!("{}_{}", tgt_safe_id, Self::sanitize_id(&edge.target_symbol));
                        let label = if let Some(line) = edge.line_number {
                            format!("L{}", line)
                        } else {
                            "".to_string()
                        };
                        mermaid.push_str(&format!("    {} -->|{}| {}\n", src_id, label, tgt_id));
                    }
                } else {
                    // Summary module-to-module edges
                    mermaid.push_str(&format!(
                        "    node_{} -->|{} edges| node_{}\n",
                        src_safe_id, dep.count, tgt_safe_id
                    ));
                }
            }
        }
        mermaid
    }

    pub fn to_json(modules: &[ModuleResponse]) -> String {
        serde_json::to_string_pretty(&json!({ "modules": modules })).unwrap_or_default()
    }

    pub fn to_html(modules: &[ModuleResponse]) -> String {
        let mermaid_code = Self::to_mermaid(modules);
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CodeMate Module Graph</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f8f9fa;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        h1 {{ color: #333; border-bottom: 2px solid #eee; padding-bottom: 10px; }}
        .controls {{ margin-bottom: 20px; color: #666; font-size: 0.9em; }}
        #graph {{ display: flex; justify-content: center; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>CodeMate Module Graph</h1>
        <div class="controls">
            Interactive dependency visualization. Zoom and pan using your browser.
        </div>
        <div id="graph">
            <pre class="mermaid">
{}
            </pre>
        </div>
    </div>
    <script>
        mermaid.initialize({{ 
            startOnLoad: true,
            theme: 'default',
            maxTextSize: 1000000,
            securityLevel: 'loose',
            flowchart: {{ 
                useMaxWidth: true, 
                htmlLabels: true,
                curve: 'basis'
            }}
        }});
    </script>
</body>
</html>"#,
            mermaid_code
        )
    }

    fn sanitize_id(id: &str) -> String {
        let sanitized = id.replace("::", "_")
            .replace("-", "_")
            .replace(".", "_")
            .replace("/", "_")
            .replace(" ", "_")
            .replace("(", "_")
            .replace(")", "_")
            .replace("[", "_")
            .replace("]", "_")
            .replace("{", "_")
            .replace("}", "_")
            .replace(">", "_")
            .replace("<", "_")
            .replace("=", "_")
            .replace("!", "_")
            .replace("@", "_")
            .replace("#", "_")
            .replace("$", "_")
            .replace("%", "_")
            .replace("^", "_")
            .replace("&", "_")
            .replace("*", "_")
            .replace("+", "_")
            .replace("|", "_")
            .replace(":", "_")
            .replace(";", "_")
            .replace("\"", "_")
            .replace("'", "_")
            .replace(",", "_");

        if sanitized.is_empty() {
            "root".to_string()
        } else {
            sanitized
        }
    }
}
