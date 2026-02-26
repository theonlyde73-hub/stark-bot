/**
 * TuiDashboard — xterm.js wrapper for interactive TUI module dashboards.
 *
 * Fetches ANSI output from the module proxy with client-side state
 * (selected row, scroll offset) and handles keyboard navigation + actions.
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { API_BASE } from '@/lib/api';
import { useGateway } from '@/hooks/useGateway';

interface ActionDef {
  key: string;
  label: string;
  action: string;
  confirm?: boolean;
  prompts?: string[];
}

interface ActionsMetadata {
  navigable: boolean;
  actions: ActionDef[];
}

interface Props {
  moduleName: string;
}

function getToken(): string | null {
  return localStorage.getItem('stark_token');
}

function authHeaders(): Record<string, string> {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function fetchAnsi(
  moduleName: string,
  width: number,
  height: number,
  selected: number,
  scroll: number,
): Promise<string> {
  const params = new URLSearchParams({
    width: String(width),
    height: String(height),
    selected: String(selected),
    scroll: String(scroll),
  });
  const resp = await fetch(
    `${API_BASE}/modules/${moduleName}/proxy/rpc/dashboard/tui?${params}`,
    { headers: authHeaders() },
  );
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  return resp.text();
}

async function fetchActionsMeta(moduleName: string): Promise<ActionsMetadata> {
  const resp = await fetch(
    `${API_BASE}/modules/${moduleName}/proxy/rpc/dashboard/tui/actions`,
    { headers: authHeaders() },
  );
  if (!resp.ok) return { navigable: false, actions: [] };
  return resp.json();
}

async function postAction(
  moduleName: string,
  action: string,
  state: { selected: number; scroll: number },
  inputs?: string[],
): Promise<{ ok: boolean; message?: string; error?: string }> {
  const resp = await fetch(
    `${API_BASE}/modules/${moduleName}/proxy/rpc/dashboard/tui/action`,
    {
      method: 'POST',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify({ action, state, inputs }),
    },
  );
  if (!resp.ok) return { ok: false, error: `HTTP ${resp.status}` };
  return resp.json();
}

export default function TuiDashboard({ moduleName }: Props) {
  const wrapperRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const stateRef = useRef({ selected: 0, scroll: 0 });
  const metaRef = useRef<ActionsMetadata>({ navigable: false, actions: [] });
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const keyHandlerRef = useRef<((e: KeyboardEvent) => void) | null>(null);
  const { on, off } = useGateway();
  const [focused, setFocused] = useState(false);
  const [promptOverlay, setPromptOverlay] = useState<{
    actionDef: ActionDef;
    values: string[];
    currentIndex: number;
  } | null>(null);

  // Render ANSI to terminal
  const renderDashboard = useCallback(async () => {
    const term = termRef.current;
    if (!term) return;
    const { selected, scroll } = stateRef.current;
    try {
      const ansi = await fetchAnsi(
        moduleName,
        term.cols,
        term.rows,
        selected,
        scroll,
      );
      term.reset();
      term.write(ansi);
    } catch {
      // silently skip failed renders
    }
  }, [moduleName]);

  // Focus the wrapper div (gives us keyboard events without xterm interference)
  const focusWrapper = useCallback(() => {
    wrapperRef.current?.focus();
  }, []);

  // Keyboard handler — processes keys for navigation and actions.
  // Uses a ref so xterm's custom key handler and the wrapper's native
  // listener always call the latest version without effect re-runs.
  useEffect(() => {
    keyHandlerRef.current = async (e: KeyboardEvent) => {
      if (promptOverlay) return;

      const meta = metaRef.current;
      const state = stateRef.current;

      if (e.key === 'ArrowUp' && meta.navigable) {
        e.preventDefault();
        if (state.selected > 0) {
          state.selected--;
          if (state.selected < state.scroll) state.scroll = state.selected;
          await renderDashboard();
        }
        return;
      }

      if (e.key === 'ArrowDown' && meta.navigable) {
        e.preventDefault();
        state.selected++;
        if (state.selected >= state.scroll + 20) state.scroll++;
        await renderDashboard();
        return;
      }

      if (e.key === 'PageUp' && meta.navigable) {
        e.preventDefault();
        state.selected = Math.max(0, state.selected - 20);
        state.scroll = Math.max(0, state.scroll - 20);
        await renderDashboard();
        return;
      }

      if (e.key === 'PageDown' && meta.navigable) {
        e.preventDefault();
        state.selected += 20;
        state.scroll += 20;
        await renderDashboard();
        return;
      }

      // Action keys
      const actionDef = meta.actions.find((a) => a.key === e.key);
      if (!actionDef) return;

      e.preventDefault();

      if (actionDef.prompts && actionDef.prompts.length > 0) {
        setPromptOverlay({ actionDef, values: [], currentIndex: 0 });
        return;
      }

      if (actionDef.confirm) {
        if (!window.confirm(`${actionDef.label}?`)) return;
      }

      await postAction(moduleName, actionDef.action, { ...state });
      await renderDashboard();
    };
  }, [moduleName, renderDashboard, promptOverlay]);

  // Initialize terminal
  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: false,
      disableStdin: true,
      theme: {
        background: '#0f172a',
        foreground: '#e2e8f0',
        cursor: '#0f172a',
        selectionBackground: '#334155',
        black: '#1e293b',
        red: '#ef4444',
        green: '#22c55e',
        yellow: '#eab308',
        blue: '#3b82f6',
        magenta: '#a855f7',
        cyan: '#06b6d4',
        white: '#f1f5f9',
        brightBlack: '#475569',
        brightRed: '#f87171',
        brightGreen: '#4ade80',
        brightYellow: '#facc15',
        brightBlue: '#60a5fa',
        brightMagenta: '#c084fc',
        brightCyan: '#22d3ee',
        brightWhite: '#f8fafc',
      },
      fontSize: 14,
      fontFamily: '"JetBrains Mono", "Fira Code", monospace',
      scrollback: 0,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(containerRef.current);
    fit.fit();

    termRef.current = term;
    fitRef.current = fit;

    // Load actions metadata
    fetchActionsMeta(moduleName).then((meta) => {
      metaRef.current = meta;
    });

    // Block xterm from capturing keys — let them fall through to the
    // wrapper div's native keydown listener instead.
    term.attachCustomKeyEventHandler(() => false);

    // Native keydown on wrapper — always calls the latest handler via ref
    const onKeyDown = (e: KeyboardEvent) => {
      keyHandlerRef.current?.(e);
    };
    wrapperRef.current?.addEventListener('keydown', onKeyDown);
    const savedWrapper = wrapperRef.current;

    // Auto-focus on mount
    focusWrapper();

    // Initial render
    renderDashboard();

    // Fallback polling every 10s (WS handles realtime updates)
    const timer = setInterval(renderDashboard, 10_000);

    // Subscribe to WS invalidation events for this module
    const handleInvalidate = (data: unknown) => {
      const d = data as { module?: string };
      if (d?.module !== moduleName) return;
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(renderDashboard, 100);
    };
    on('module.tui_invalidate', handleInvalidate);

    // Resize observer
    const ro = new ResizeObserver(() => {
      fit.fit();
      renderDashboard();
    });
    ro.observe(containerRef.current);

    return () => {
      clearInterval(timer);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      off('module.tui_invalidate', handleInvalidate);
      savedWrapper?.removeEventListener('keydown', onKeyDown);
      ro.disconnect();
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
  }, [moduleName, renderDashboard, on, off, focusWrapper]);

  // Re-focus wrapper after prompt overlay closes
  useEffect(() => {
    if (!promptOverlay) {
      focusWrapper();
    }
  }, [promptOverlay, focusWrapper]);

  // Submit prompt overlay
  const handlePromptSubmit = useCallback(
    async (allValues: string[]) => {
      if (!promptOverlay) return;
      setPromptOverlay(null);
      const state = stateRef.current;
      await postAction(moduleName, promptOverlay.actionDef.action, { ...state }, allValues);
      await renderDashboard();
    },
    [moduleName, promptOverlay, renderDashboard],
  );

  return (
    <div
      ref={wrapperRef}
      className={`relative w-full h-full rounded transition-shadow ${focused ? 'ring-1 ring-stark-500/50' : ''}`}
      tabIndex={0}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
      onClick={focusWrapper}
      style={{ outline: 'none' }}
    >
      <div ref={containerRef} className="w-full h-full" />

      {/* Click-to-focus hint when unfocused */}
      {!focused && !promptOverlay && (
        <div className="absolute inset-0 flex items-end justify-center pb-3 pointer-events-none">
          <span className="text-xs text-slate-500 bg-slate-900/80 px-3 py-1 rounded-full">
            Click to interact
          </span>
        </div>
      )}

      {/* Prompt overlay */}
      {promptOverlay && (
        <PromptOverlay
          prompts={promptOverlay.actionDef.prompts!}
          onSubmit={handlePromptSubmit}
          onCancel={() => setPromptOverlay(null)}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Prompt overlay for actions that need user input
// ---------------------------------------------------------------------------

function PromptOverlay({
  prompts,
  onSubmit,
  onCancel,
}: {
  prompts: string[];
  onSubmit: (values: string[]) => void;
  onCancel: () => void;
}) {
  const [values, setValues] = useState<string[]>(prompts.map(() => ''));
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);

  useEffect(() => {
    inputRefs.current[0]?.focus();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onCancel();
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(values);
  };

  return (
    <div
      className="absolute inset-0 bg-black/60 flex items-center justify-center z-10"
      onKeyDown={handleKeyDown}
    >
      <form
        onSubmit={handleSubmit}
        className="bg-slate-800 border border-slate-600 rounded-lg p-6 min-w-[320px] space-y-4"
      >
        {prompts.map((label, i) => (
          <div key={i}>
            <label className="block text-sm text-slate-300 mb-1">{label}</label>
            <input
              ref={(el) => { inputRefs.current[i] = el; }}
              type="text"
              value={values[i]}
              onChange={(e) => {
                const next = [...values];
                next[i] = e.target.value;
                setValues(next);
              }}
              className="w-full px-3 py-2 bg-slate-900 border border-slate-600 rounded text-white text-sm focus:outline-none focus:border-stark-500"
              autoComplete="off"
            />
          </div>
        ))}
        <div className="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-1.5 text-sm text-slate-400 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="px-4 py-1.5 text-sm bg-stark-600 hover:bg-stark-500 text-white rounded transition-colors"
          >
            Submit
          </button>
        </div>
      </form>
    </div>
  );
}
