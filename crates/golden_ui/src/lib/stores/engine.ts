import { derived, writable, type Readable, type Writable } from "svelte/store";

type NodeId = number | string;

type EventTime = {
  tick: number;
  micro: number;
  seq: number;
};

type NodeMeta = {
  label: string;
  decl_id: string;
  short_name: string;
  enabled: boolean;
  semantics?: { intent?: string | null } | null;
};

type NodeDto = {
  node_id: NodeId;
  node_type: string;
  meta: NodeMeta;
  children: NodeId[];
  data?: { kind?: string };
};

type ParamDto = {
  param_node_id: NodeId;
  value: unknown;
  constraints?: unknown;
  semantics?: { intent?: string | null } | null;
};

type Event = {
  node: NodeId;
  time: EventTime;
  kind: Record<string, unknown> | null;
};

type EngineStatus = {
  state: "disconnected" | "connecting" | "connected";
  detail: string;
};

type Envelope = {
  msg: string;
  req_id: string | null;
  payload: unknown;
};

const initialStatus: EngineStatus = {
  state: "disconnected",
  detail: ""
};

const nodes: Writable<NodeDto[]> = writable([]);
const params: Writable<ParamDto[]> = writable([]);
const events: Writable<Event[]> = writable([]);
const status: Writable<EngineStatus> = writable(initialStatus);
const selection: Writable<{ nodeId: NodeId | null }> = writable({ nodeId: null });
const eventTime: Writable<EventTime> = writable({ tick: 0, micro: 0, seq: 0 });

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

const nodeById: Readable<Map<NodeId, NodeDto>> = derived(nodes, ($nodes) => {
  const map = new Map<NodeId, NodeDto>();
  for (const node of $nodes) {
    map.set(node.node_id, node);
  }
  return map;
});

const paramById: Readable<Map<NodeId, ParamDto>> = derived(params, ($params) => {
  const map = new Map<NodeId, ParamDto>();
  for (const param of $params) {
    map.set(param.param_node_id, param);
  }
  return map;
});

const selectedNode: Readable<NodeDto | null> = derived(
  [selection, nodeById],
  ([$selection, $nodeById]) => ($selection.nodeId ? $nodeById.get($selection.nodeId) ?? null : null)
);

function wsUrl() {
  const envServer = import.meta.env.VITE_GOLDEN_SERVER as string | undefined;
  const isDev = window.location.port === "5173";
  const base = envServer?.length
    ? envServer
    : isDev
    ? `${window.location.protocol}//${window.location.hostname}:9010`
    : window.location.href;
  const url = new URL("/ws", base);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString();
}

function send(envelope: Envelope) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    return;
  }
  socket.send(JSON.stringify(envelope));
}

function requestSnapshot() {
  send({
    msg: "GetSnapshot",
    req_id: null,
    payload: { scope: { mode: "Root" }, include_schema: true }
  });
}

function subscribe(from: EventTime) {
  send({
    msg: "Subscribe",
    req_id: null,
    payload: { scope: { mode: "Root" }, from }
  });
}

function connect() {
  if (socket) {
    return;
  }

  const url = wsUrl();
  status.set({ state: "connecting", detail: url });
  socket = new WebSocket(url);

  socket.addEventListener("open", () => {
    status.set({ state: "connected", detail: url });
    requestSnapshot();
  });

  socket.addEventListener("message", (event) => {
    const envelope = JSON.parse(event.data) as Envelope & { payload: any };
    if (envelope.msg === "Snapshot") {
      nodes.set(envelope.payload.nodes ?? []);
      params.set(envelope.payload.params ?? []);
      eventTime.set(envelope.payload.as_of);
      subscribe(envelope.payload.as_of);
    }
    if (envelope.msg === "EventBatch") {
      events.update((current) => {
        const next = [...(envelope.payload.events ?? []), ...current];
        return next.slice(0, 200);
      });
      const last = envelope.payload.events?.at(-1);
      if (last?.time) {
        eventTime.set(last.time);
      }
    }
  });

  socket.addEventListener("close", () => {
    socket = null;
    status.set({ state: "disconnected", detail: "" });
    if (!reconnectTimer) {
      reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        connect();
      }, 1500);
    }
  });
}

function setSelection(nodeId: NodeId) {
  selection.set({ nodeId });
}

function setParam(paramNodeId: NodeId, value: unknown, propagation = "Immediate") {
  send({
    msg: "SetParam",
    req_id: null,
    payload: {
      edit_session_id: null,
      param_node_id: paramNodeId,
      value,
      propagation
    }
  });
}

export const engineStore = {
  nodes,
  params,
  events,
  status,
  selection,
  eventTime,
  nodeById,
  paramById,
  selectedNode,
  connect,
  setSelection,
  setParam
};
