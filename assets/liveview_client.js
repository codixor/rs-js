const intercept_link_redirects = false;

const WS_CONFIG = Object.freeze({
  heartbeatIntervalMs: 30_000,
  heartbeatTimeoutMs: 45_000,
  reconnectInitialDelayMs: 500,
  reconnectMaxDelayMs: 10_000,
  reconnectFactor: 1.8,
  reconnectJitterRatio: 0.2,
  maxQueuedMessages: 1024,
  maxBatchPerFlush: 128,
});

function resolvePortalRuntime() {
  const root =
    typeof window !== "undefined"
      ? window
      : typeof globalThis !== "undefined"
        ? globalThis
        : null;

  return root && root.__DX_PORTAL_RUNTIME__ ? root.__DX_PORTAL_RUNTIME__ : null;
}

function runPortalRuntimeAction(args) {
  if (!args) {
    return;
  }

  const runtime = resolvePortalRuntime();
  if (!runtime || typeof runtime.dispatch !== "function") {
    return;
  }

  runtime.dispatch({
    kind: typeof args.action === "string" ? args.action : "",
    host_id: typeof args.hostKey === "string" ? args.hostKey : null,
    parent_selector:
      typeof args.parentSelector === "string" ? args.parentSelector : null,
    surface_id: typeof args.surfaceId === "string" ? args.surfaceId : null,
  });
}

function runStyleRuntimeAction(args) {
  const api = window.__DX_LIVEVIEW__ || (window.__DX_LIVEVIEW__ = {});
  if (typeof api.styleRuntime === "function") {
    api.styleRuntime(args);
  }
}

function runRootHtmlRuntimeAction(args) {
  const runtime = globalThis.__DX_ROOT_HTML__;
  if (!runtime || typeof runtime.syncRootHtmlAttrs !== "function") {
    return false;
  }

  return runtime.syncRootHtmlAttrs(args);
}

function runHeadRuntimeAction(args) {
  if (!args || typeof document === "undefined") {
    return;
  }

  const head = document.head || document.documentElement;
  const tag = typeof args.tag === "string" ? args.tag : "";
  if (!head || !tag) {
    return;
  }

  const element = document.createElement(tag);
  const attrs =
    args.attrs && typeof args.attrs === "object" && !Array.isArray(args.attrs)
      ? args.attrs
      : {};

  for (const [name, value] of Object.entries(attrs)) {
    if (value === undefined || value === null) {
      continue;
    }
    element.setAttribute(name, String(value));
  }

  if (typeof args.text === "string") {
    element.textContent = args.text;
  }

  head.appendChild(element);
}

function readLiveviewSession() {
  try {
    const raw = sessionStorage.getItem("liveview");
    return raw ? JSON.parse(raw) : null;
  } catch (_err) {
    return null;
  }
}

function writeLiveviewSession(session) {
  try {
    if (session == null) {
      sessionStorage.removeItem("liveview");
      return;
    }
    sessionStorage.setItem("liveview", JSON.stringify(session));
  } catch (_err) {
    // Intentionally ignored.
  }
}

function resolveHistoryRuntime() {
  if (!window.__DX_HISTORY_RUNTIME__) {
    window.__DX_HISTORY_RUNTIME__ = {
      requestId: null,
      listenerInstalled: false,
      navigationMode: "url",
      currentRoute: null,
    };
  }

  return window.__DX_HISTORY_RUNTIME__;
}

function currentBrowserUrl() {
  const path = document.location.pathname + document.location.search;
  const hash = document.location.hash;
  return hash ? path + hash : path;
}

function normalizeHistoryNavigationMode(value) {
  return value === "dom_view" ? "dom_view" : "url";
}

function resolveDomViewRoute(state) {
  const index = state && Number.isInteger(state.index) ? state.index : null;
  const session = readLiveviewSession();
  const routes =
    session && session.routes && typeof session.routes === "object"
      ? session.routes
      : null;

  if (index !== null && routes && typeof routes[String(index)] === "string") {
    return routes[String(index)];
  }

  const runtime = resolveHistoryRuntime();
  return typeof runtime.currentRoute === "string" && runtime.currentRoute
    ? runtime.currentRoute
    : currentBrowserUrl();
}

function currentLiveviewRouteForMode(mode, state) {
  if (mode === "dom_view") {
    return resolveDomViewRoute(state);
  }
  return currentBrowserUrl();
}

function runHistoryRuntimeAction(ipc, args, requestId, respond) {
  const action = typeof args.action === "string" ? args.action : "";
  const navigationMode = normalizeHistoryNavigationMode(args.navigation_mode);
  const runtime = resolveHistoryRuntime();
  runtime.navigationMode = navigationMode;

  switch (action) {
    case "back":
      history.back();
      return;
    case "forward":
      history.forward();
      return;
    case "push":
      runtime.currentRoute = String(args.route || "/");
      history.pushState(
        args.state ?? null,
        "",
        navigationMode === "dom_view" ? currentBrowserUrl() : runtime.currentRoute,
      );
      writeLiveviewSession(args.session ?? null);
      return;
    case "replace":
      runtime.currentRoute = String(args.route || "/");
      history.replaceState(
        args.state ?? null,
        "",
        navigationMode === "dom_view" ? currentBrowserUrl() : runtime.currentRoute,
      );
      writeLiveviewSession(args.session ?? null);
      return;
    case "external":
      if (typeof args.url === "string" && args.url) {
        location.href = args.url;
      }
      return;
    case "initPopstate": {
      runtime.requestId = requestId;
      runtime.navigationMode = navigationMode;
      const route = currentLiveviewRouteForMode(navigationMode, history.state ?? null);
      runtime.currentRoute = route;

      if (!runtime.listenerInstalled) {
        window.addEventListener("popstate", (event) => {
          const current = resolveHistoryRuntime();
          if (current.requestId == null) {
            return;
          }
          const route = currentLiveviewRouteForMode(
            current.navigationMode,
            event.state ?? null,
          );
          current.currentRoute = route;
          ipc.pushQueryValue(current.requestId, [
            route,
            event.state ?? null,
          ]);
        });
        runtime.listenerInstalled = true;
      }

      respond([
        route,
        history.state ?? null,
        readLiveviewSession(),
        history.length,
      ]);
      return;
    }
    default:
      return;
  }
}

function registerLiveviewQueries(ipc) {
  if (!ipc) {
    return null;
  }

  const api = window.__DX_LIVEVIEW__ || (window.__DX_LIVEVIEW__ = {});
  api.queries = api.queries || Object.create(null);

  const handlers = {
    focusMain: () => {
      if (ipc.root && typeof ipc.root.focus === "function") {
        ipc.root.focus();
      }
    },
    portalRuntime: ({ args }) => {
      runPortalRuntimeAction(args);
    },
    styleRuntime: ({ args }) => {
      runStyleRuntimeAction(args);
    },
    rootHtmlRuntime: ({ args }) => {
      runRootHtmlRuntimeAction(args);
    },
    headRuntime: ({ args }) => {
      runHeadRuntimeAction(args);
    },
    historyRuntime: ({ ipc, args, requestId, respond }) => {
      runHistoryRuntimeAction(ipc, args, requestId, respond);
    },
    elementRuntime: ({ args, respond }) => {
      if (!args || typeof respond !== "function") {
        return;
      }

      const action = typeof args.action === "string" ? args.action : "";
      const elementId = Number.isInteger(args.elementId) ? args.elementId : null;
      if (elementId == null || !window.interpreter) {
        respond(null);
        return;
      }

      switch (action) {
        case "getScrollOffset":
          respond([
            window.interpreter.getScrollLeft(elementId),
            window.interpreter.getScrollTop(elementId),
          ]);
          return;
        case "getScrollSize":
          respond([
            window.interpreter.getScrollWidth(elementId),
            window.interpreter.getScrollHeight(elementId),
          ]);
          return;
        case "getClientRect":
          respond(window.interpreter.getClientRect(elementId));
          return;
        case "scrollTo":
          respond(!!window.interpreter.scrollTo(elementId, args.options ?? null));
          return;
        case "scroll":
          respond(
            !!window.interpreter.scroll(
              elementId,
              Number(args.x ?? 0),
              Number(args.y ?? 0),
              args.behavior ?? null
            )
          );
          return;
        case "setFocus":
          respond(!!window.interpreter.setFocus(elementId, !!args.focus));
          return;
        default:
          respond(null);
      }
    },
  };

  Object.assign(api.queries, handlers);
  window.LiveViewQueries = api.queries;
  ipc.registerQueries(handlers);
  return ipc;
}

function main() {
  const api = window.__DX_LIVEVIEW__ || (window.__DX_LIVEVIEW__ = {});
  if (api.mux && typeof api.mux.installGlobalWebSocketMux === "function") {
    api.mux.installGlobalWebSocketMux();
  }
  window.__dxLiveviewMuxInstalled = true;

  const ipc = api.startIpc ? api.startIpc("main", WS_ADDR, WS_CONFIG) : null;
  if (!ipc) {
    return;
  }

  registerLiveviewQueries(ipc);
}

main();
