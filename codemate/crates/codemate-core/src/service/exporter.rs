use crate::service::models::ModuleResponse;
use serde_json::json;

pub struct ModuleGraphExporter;

impl ModuleGraphExporter {
    pub fn to_dot(modules: &[ModuleResponse]) -> String {
        let mut dot = String::from("digraph ModuleGraph {\n");
        dot.push_str("  node [shape=box, fontname=\"Arial\"];\n");
        dot.push_str("  rankdir=LR;\n\n");

        for m_resp in modules {
            let label = format!("{} ({})", m_resp.module.name, m_resp.module.project_type.as_str());
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", m_resp.module.id, label));

            for dep in &m_resp.dependencies {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                    m_resp.module.id, dep.target_id, dep.count
                ));
            }
        }

        dot.push_str("}\n");
        dot
    }

    pub fn to_mermaid(modules: &[ModuleResponse]) -> String {
        let mut mermaid = String::from("graph TD\n");
        for m_resp in modules {
            // Mermaid uses brackets for node shapes: [box], (rounded), [[subroutine]], etc.
            // We'll use [box] for modules.
            for dep in &m_resp.dependencies {
                mermaid.push_str(&format!(
                    "    {}[\"{}\"] -->|{}| {}[\"{}\"]\n",
                    Self::sanitize_mermaid_id(&m_resp.module.id),
                    m_resp.module.name,
                    dep.count,
                    Self::sanitize_mermaid_id(&dep.target_id),
                    dep.target_name
                ));
            }
            
            if m_resp.dependencies.is_empty() {
                 mermaid.push_str(&format!(
                    "    {}[\"{}\"]\n",
                    Self::sanitize_mermaid_id(&m_resp.module.id),
                    m_resp.module.name
                ));
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

    fn sanitize_mermaid_id(id: &str) -> String {
        let sanitized = id.replace("::", "_").replace("-", "_").replace(".", "_").replace("/", "_");
        if sanitized.is_empty() {
            "root".to_string()
        } else {
            sanitized
        }
    }
}
