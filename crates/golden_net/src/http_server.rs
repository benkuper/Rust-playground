use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Clone, Debug)]
pub struct HttpServerConfig {
    pub addr: SocketAddr,
}

pub async fn start_http_server(config: HttpServerConfig) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.addr).await?;
    loop {
        let (mut stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(err) = handle_connection(&mut stream).await {
                eprintln!("http error: {err}");
            }
        });
    }
}

async fn handle_connection(stream: &mut tokio::net::TcpStream) -> anyhow::Result<()> {
    let mut buffer = [0u8; 4096];
    let size = stream.read(&mut buffer).await?;
    if size == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..size]);
    let line = request.lines().next().unwrap_or_default();
    let path = line.split_whitespace().nth(1).unwrap_or("/");

    let (status, content_type, body) = match path {
        "/app.js" => (
            "200 OK",
            "text/javascript; charset=utf-8",
            JS_BUNDLE.as_bytes(),
        ),
        "/app.css" => ("200 OK", "text/css; charset=utf-8", CSS_BUNDLE.as_bytes()),
        _ => ("200 OK", "text/html; charset=utf-8", HTML_BUNDLE.as_bytes()),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {len}\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n",
        status = status,
        len = body.len(),
        content_type = content_type
    );

    stream.write_all(response.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}

const HTML_BUNDLE: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;600;700&display=swap" rel="stylesheet" />
    <link rel="stylesheet" href="/app.css" />
    <title>Golden Core Outliner</title>
  </head>
  <body>
    <div id="app"></div>
    <script src="/app.js"></script>
  </body>
</html>"#;

const CSS_BUNDLE: &str = r#":root {
  color-scheme: only light;
  --bg: #f3efe7;
  --bg-accent: #f2e3c8;
  --panel: #fff7ec;
  --panel-2: #fff1dc;
  --ink: #1b1a17;
  --muted: #6b6256;
  --accent: #d07b2c;
  --accent-2: #2c6bd0;
  --outline: rgba(27, 26, 23, 0.12);
  font-family: "Space Grotesk", system-ui, sans-serif;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background:
    radial-gradient(circle at top, var(--bg-accent), var(--bg)),
    repeating-linear-gradient(120deg, rgba(255, 255, 255, 0.25) 0, rgba(255, 255, 255, 0.25) 2px, transparent 2px, transparent 8px);
  color: var(--ink);
  min-height: 100vh;
}
main {
  max-width: 1100px;
  margin: 48px auto;
  padding: 0 24px 64px;
}
header {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 32px;
}
header h1 {
  font-size: clamp(2.2rem, 3vw, 3.3rem);
  margin: 0;
  letter-spacing: -0.03em;
}
header p {
  margin: 0;
  color: var(--muted);
  max-width: 640px;
}
.outliner {
  background: var(--panel);
  border: 1px solid var(--outline);
  border-radius: 18px;
  padding: 24px;
  box-shadow: 0 20px 40px rgba(27, 26, 23, 0.08);
}
.status {
  font-size: 0.95rem;
  color: var(--muted);
  margin-bottom: 16px;
}
.node-row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 12px;
  padding: 6px 0;
}
.node-kind {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}
.node-label { font-weight: 600; }
.node-value {
  font-size: 0.9rem;
  color: var(--accent-2);
}
.node-meta {
  display: flex;
  gap: 10px;
  align-items: center;
  justify-content: flex-end;
}
.badge {
  font-size: 0.72rem;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--panel-2);
  border: 1px solid var(--outline);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted);
}
.tree { margin: 0; padding: 0; list-style: none; }
.tree li { position: relative; padding-left: 24px; }
.tree li::before {
  content: "";
  position: absolute;
  left: 8px;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--outline);
}
.tree li::after {
  content: "";
  position: absolute;
  left: 8px;
  top: 18px;
  width: 12px;
  height: 1px;
  background: var(--outline);
}
.tree li:last-child::before { height: 18px; }
.param-control {
  margin: 8px 0 14px 0;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--outline);
  background: #fffdf7;
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 12px;
  align-items: center;
}
.param-control label {
  font-size: 0.85rem;
  color: var(--muted);
}
.param-inputs {
  display: grid;
  gap: 8px;
  align-items: center;
  justify-items: end;
}
.param-inputs input[type="number"],
.param-inputs input[type="text"] {
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid var(--outline);
  background: white;
  width: 120px;
  font-family: inherit;
}
.param-inputs input[type="range"] {
  width: 180px;
}
.toggle {
  display: inline-flex;
  gap: 10px;
  align-items: center;
  padding: 6px 8px;
  border-radius: 999px;
  background: var(--panel-2);
  border: 1px solid var(--outline);
}
.toggle input {
  width: 40px;
  height: 20px;
}
@media (max-width: 720px) {
  .node-row { grid-template-columns: 1fr; gap: 4px; }
  .node-meta { justify-content: flex-start; }
  .param-control { grid-template-columns: 1fr; justify-items: start; }
  .param-inputs { justify-items: start; }
}
"#;

const JS_BUNDLE: &str = r#"(() => {
  const root = document.getElementById('app');

  const state = {
    nodes: [],
    params: [],
    status: 'Disconnected'
  };

  let lastEventTime = { tick: 0, micro: 0, seq: 0 };
  const interactionUntil = new Map();

  const nodeById = () => {
    const map = new Map();
    state.nodes.forEach((node) => map.set(node.node_id, node));
    return map;
  };

  const paramById = () => {
    const map = new Map();
    state.params.forEach((param) => map.set(param.param_node_id, param));
    return map;
  };

  const decodeValue = (value) => {
    if (value && typeof value === 'object') {
      const key = Object.keys(value)[0];
      return { kind: key, value: value[key] };
    }
    return { kind: 'Unknown', value };
  };

  const decodeConstraints = (constraints) => {
    if (constraints && typeof constraints === 'object') {
      const key = Object.keys(constraints)[0];
      return { kind: key, value: constraints[key] };
    }
    return { kind: 'None', value: null };
  };

  const valueText = (param) => {
    if (!param) return '';
    const decoded = decodeValue(param.value);
    return `${decoded.kind} ${JSON.stringify(decoded.value)}`;
  };

  const renderParamControl = (param) => {
    if (!param) return '';
    const decoded = decodeValue(param.value);
    const constraints = decodeConstraints(param.constraints);
    const disabled = param.read_only ? 'disabled' : '';
    const id = param.param_node_id;

    if (decoded.kind === 'Bool') {
      return `
        <div class="param-control">
          <label>Toggle</label>
          <div class="param-inputs">
            <span class="toggle">
              <input type="checkbox" data-param-id="${id}" data-kind="Bool" ${decoded.value ? 'checked' : ''} ${disabled} />
            </span>
          </div>
        </div>
      `;
    }

    if (decoded.kind === 'String') {
      return `
        <div class="param-control">
          <label>Text</label>
          <div class="param-inputs">
            <input type="text" value="${decoded.value ?? ''}" data-param-id="${id}" data-kind="String" ${disabled} />
          </div>
        </div>
      `;
    }

    if (decoded.kind === 'Int' || decoded.kind === 'Float') {
      const constraintValue = constraints.kind === decoded.kind && constraints.value ? constraints.value : {};
      const min = constraintValue.min ?? null;
      const max = constraintValue.max ?? null;
      const step = constraintValue.step != null
        ? constraintValue.step
        : decoded.kind === 'Float' ? 0.01 : 1;
      const rangeMin = min != null ? min : decoded.kind === 'Float' ? 0 : 0;
      const rangeMax = max != null ? max : decoded.kind === 'Float' ? 1 : 100;
      const value = decoded.value ?? 0;
      const kind = decoded.kind;
      return `
        <div class="param-control">
          <label>${kind} (${rangeMin} to ${rangeMax})</label>
          <div class="param-inputs">
            <input type="range" min="${rangeMin}" max="${rangeMax}" step="${step}" value="${value}" data-param-id="${id}" data-kind="${kind}" data-role="range" ${disabled} />
            <input type="number" min="${rangeMin}" max="${rangeMax}" step="${step}" value="${value}" data-param-id="${id}" data-kind="${kind}" data-role="number" ${disabled} />
          </div>
        </div>
      `;
    }

    return `
      <div class="param-control">
        <label>Read only</label>
        <div class="param-inputs">
          <span class="badge">${decoded.kind}</span>
        </div>
      </div>
    `;
  };

  const renderTree = (node) => {
    if (!node) return '';
    const param = paramById().get(node.node_id);
    const children = (node.children || [])
      .map((childId) => renderTree(nodeById().get(childId)))
      .join('');
    return `
      <li>
        <div class="node-row">
          <span class="node-kind">${node.node_type}</span>
          <span class="node-label">${node.meta.label}</span>
          <span class="node-value" data-param-id="${param ? param.param_node_id : ''}">${valueText(param)}</span>
        </div>
        ${param ? renderParamControl(param) : ''}
        ${children ? `<ul class="tree">${children}</ul>` : ''}
      </li>
    `;
  };

  const render = () => {
    const rootNode = state.nodes.find((node) => node.meta.decl_id === 'root');
    root.innerHTML = `
      <main>
        <header>
          <h1>Golden Core Outliner</h1>
          <p>Live hierarchy view shared by Tauri and browsers, with editable parameters.</p>
        </header>
        <section class="outliner">
          <div class="status">${state.status}</div>
          ${rootNode ? `<ul class="tree">${renderTree(rootNode)}</ul>` : '<p>No root node yet.</p>'}
        </section>
      </main>
    `;
  };

  const ws = new WebSocket('ws://localhost:9001');

  const sendSubscribe = () => {
    ws.send(JSON.stringify({
      msg: 'Subscribe',
      payload: { scope: { mode: 'Root' }, from: lastEventTime }
    }));
  };

  const sendSetParam = (paramId, kind, value) => {
    const payload = {
      edit_session_id: null,
      param_node_id: Number(paramId),
      value: { [kind]: value },
      propagation: 'EndOfTick'
    };
    ws.send(JSON.stringify({ msg: 'SetParam', payload }));
  };

  const isInteracting = (paramId) => {
    const until = interactionUntil.get(paramId);
    return typeof until === 'number' && until > Date.now();
  };

  const updateParamDisplay = (paramId, value) => {
    const valueEl = root.querySelector(`.node-value[data-param-id="${paramId}"]`);
    if (valueEl) {
      const decoded = decodeValue(value);
      valueEl.textContent = `${decoded.kind} ${JSON.stringify(decoded.value)}`;
    }

    if (isInteracting(paramId)) return;

    const inputs = root.querySelectorAll(`input[data-param-id="${paramId}"]`);
    inputs.forEach((input) => {
      const kind = input.dataset.kind;
      if (kind === 'Bool') {
        input.checked = Boolean(decodeValue(value).value);
      } else {
        const decoded = decodeValue(value).value;
        if (decoded !== undefined && decoded !== null) {
          input.value = decoded;
        }
      }
    });
  };

  root.addEventListener('input', (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    const paramId = target.dataset.paramId;
    if (!paramId) return;
    const kind = target.dataset.kind;
    let value = target.value;
    if (kind === 'Bool') {
      value = target.checked;
    } else if (kind === 'Int') {
      value = Number.parseInt(target.value, 10);
      if (Number.isNaN(value)) return;
    } else if (kind === 'Float') {
      value = Number.parseFloat(target.value);
      if (Number.isNaN(value)) return;
    }

    interactionUntil.set(paramId, Date.now() + 200);
    sendSetParam(paramId, kind, value);

    if (target.dataset.role === 'range') {
      const numberInput = root.querySelector(`input[data-param-id="${paramId}"][data-role="number"]`);
      if (numberInput) numberInput.value = target.value;
    }
    if (target.dataset.role === 'number') {
      const rangeInput = root.querySelector(`input[data-param-id="${paramId}"][data-role="range"]`);
      if (rangeInput) rangeInput.value = target.value;
    }
  });

  ws.addEventListener('open', () => {
    state.status = 'Connected to ws://localhost:9001';
    ws.send(JSON.stringify({ msg: 'GetSnapshot', payload: { scope: { mode: 'Root' }, include_schema: true } }));
    render();
  });

  ws.addEventListener('close', () => {
    state.status = 'Disconnected';
    render();
  });

  ws.addEventListener('message', (event) => {
    const data = JSON.parse(event.data);
    if (data.msg === 'Snapshot') {
      state.nodes = data.payload.nodes ?? [];
      state.params = data.payload.params ?? [];
      lastEventTime = data.payload.as_of ?? lastEventTime;
      render();
      sendSubscribe();
      return;
    }

    if (data.msg === 'EventBatch') {
      const events = data.payload.events ?? [];
      let needsResync = false;
      events.forEach((event) => {
        lastEventTime = event.time ?? lastEventTime;
        if (!event.kind || typeof event.kind !== 'object') {
          needsResync = true;
          return;
        }
        const kind = Object.keys(event.kind)[0];
        const payload = event.kind[kind];

        if (kind === 'ParamChanged') {
          const param = state.params.find((p) => p.param_node_id === payload.param);
          if (param) {
            param.value = payload.value;
          }
          updateParamDisplay(payload.param, payload.value);
          return;
        }

        if (kind === 'MetaChanged') {
          const node = state.nodes.find((n) => n.node_id === payload.node);
          if (node && payload.patch) {
            const patch = payload.patch;
            if (patch.enabled !== undefined) node.meta.enabled = patch.enabled;
            if (patch.label !== undefined) node.meta.label = patch.label;
            if (patch.description !== undefined) node.meta.description = patch.description;
            if (patch.tags !== undefined) node.meta.tags = patch.tags;
            if (patch.semantics !== undefined) node.meta.semantics = patch.semantics;
            if (patch.presentation !== undefined) node.meta.presentation = patch.presentation;
          }
          return;
        }

        needsResync = true;
      });

      if (needsResync) {
        ws.send(JSON.stringify({ msg: 'GetSnapshot', payload: { scope: { mode: 'Root' }, include_schema: false } }));
      }
    }
  });

  render();
})();
"#;
