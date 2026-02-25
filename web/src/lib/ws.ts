import type { WsMessage } from '../types/api';
import { getToken } from './auth';

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface ChatSession {
  id: string;
  messages: ChatMessage[];
  title?: string;
  createdAt: number;
}

const CHAT_SESSIONS_KEY = 'zeroclaw_chat_sessions';
const CURRENT_SESSION_KEY = 'zeroclaw_current_session';
const MAX_HISTORY_MESSAGES = 50;

export type WsMessageHandler = (msg: WsMessage) => void;
export type WsOpenHandler = () => void;
export type WsCloseHandler = (ev: CloseEvent) => void;
export type WsErrorHandler = (ev: Event) => void;

export interface WebSocketClientOptions {
  /** Base URL override. Defaults to current host with ws(s) protocol. */
  baseUrl?: string;
  /** Delay in ms before attempting reconnect. Doubles on each failure up to maxReconnectDelay. */
  reconnectDelay?: number;
  /** Maximum reconnect delay in ms. */
  maxReconnectDelay?: number;
  /** Set to false to disable auto-reconnect. Default true. */
  autoReconnect?: boolean;
}

const DEFAULT_RECONNECT_DELAY = 1000;
const MAX_RECONNECT_DELAY = 30000;

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private currentDelay: number;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionallyClosed = false;
  private sessionId: string;

  public onMessage: WsMessageHandler | null = null;
  public onOpen: WsOpenHandler | null = null;
  public onClose: WsCloseHandler | null = null;
  public onError: WsErrorHandler | null = null;

  private readonly baseUrl: string;
  private readonly reconnectDelay: number;
  private readonly maxReconnectDelay: number;
  private readonly autoReconnect: boolean;

  constructor(options: WebSocketClientOptions = {}) {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.baseUrl =
      options.baseUrl ?? `${protocol}//${window.location.host}`;
    this.reconnectDelay = options.reconnectDelay ?? DEFAULT_RECONNECT_DELAY;
    this.maxReconnectDelay = options.maxReconnectDelay ?? MAX_RECONNECT_DELAY;
    this.autoReconnect = options.autoReconnect ?? true;
    this.currentDelay = this.reconnectDelay;
    this.sessionId = this.loadCurrentSessionId();
  }

  // ---------------------------------------------------------------------------
  // Session management
  // ---------------------------------------------------------------------------

  private loadCurrentSessionId(): string {
    const stored = localStorage.getItem(CURRENT_SESSION_KEY);
    if (stored) {
      try {
        const id = JSON.parse(stored);
        if (id && typeof id === 'string') return id;
      } catch {
        // Invalid JSON
      }
    }
    const newId = crypto.randomUUID();
    localStorage.setItem(CURRENT_SESSION_KEY, JSON.stringify(newId));
    return newId;
  }

  getSessions(): ChatSession[] {
    const stored = localStorage.getItem(CHAT_SESSIONS_KEY);
    if (stored) {
      try {
        const sessions = JSON.parse(stored) as ChatSession[];
        return sessions.sort((a, b) => b.createdAt - a.createdAt);
      } catch {
        return [];
      }
    }
    return [];
  }

  getCurrentSession(): ChatSession | null {
    const sessions = this.getSessions();
    return sessions.find(s => s.id === this.sessionId) || null;
  }

  setCurrentSession(sessionId: string): void {
    this.sessionId = sessionId;
    localStorage.setItem(CURRENT_SESSION_KEY, JSON.stringify(sessionId));
  }

  getHistory(): ChatMessage[] {
    const session = this.getCurrentSession();
    return session?.messages || [];
  }

  saveHistory(messages: ChatMessage[]): void {
    const trimmed = messages.slice(-MAX_HISTORY_MESSAGES);
    const firstMsg = messages[0];
    const title = firstMsg?.content 
      ? firstMsg.content.slice(0, 50) + (firstMsg.content.length > 50 ? '...' : '')
      : 'New Chat';
    
    const sessions = this.getSessions();
    const existingIndex = sessions.findIndex(s => s.id === this.sessionId);
    const existingSession = existingIndex >= 0 ? sessions[existingIndex] : null;
    
    const session: ChatSession = {
      id: this.sessionId,
      messages: trimmed,
      title,
      createdAt: existingSession?.createdAt || Date.now(),
    };
    
    if (existingIndex >= 0) {
      sessions[existingIndex] = session;
    } else {
      sessions.push(session);
    }
    
    localStorage.setItem(CHAT_SESSIONS_KEY, JSON.stringify(sessions));
  }

  clearHistory(): void {
    const sessions = this.getSessions().filter(s => s.id !== this.sessionId);
    localStorage.setItem(CHAT_SESSIONS_KEY, JSON.stringify(sessions));
    
    this.sessionId = crypto.randomUUID();
    localStorage.setItem(CURRENT_SESSION_KEY, JSON.stringify(this.sessionId));
  }

  deleteSession(sessionId: string): void {
    const sessions = this.getSessions().filter(s => s.id !== sessionId);
    localStorage.setItem(CHAT_SESSIONS_KEY, JSON.stringify(sessions));
    
    if (sessionId === this.sessionId) {
      this.sessionId = crypto.randomUUID();
      localStorage.setItem(CURRENT_SESSION_KEY, JSON.stringify(this.sessionId));
    }
  }

  getSessionId(): string {
    return this.sessionId;
  }

  /** Open the WebSocket connection. */
  connect(): void {
    this.intentionallyClosed = false;
    this.clearReconnectTimer();

    const token = getToken();
    const url = `${this.baseUrl}/ws/chat${token ? `?token=${encodeURIComponent(token)}` : ''}`;

    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this.currentDelay = this.reconnectDelay;
      this.onOpen?.();
    };

    this.ws.onmessage = (ev: MessageEvent) => {
      try {
        const msg = JSON.parse(ev.data) as WsMessage;
        this.handleMessage(msg);
      } catch {
        // Ignore non-JSON frames
      }
    };

    this.ws.onclose = (ev: CloseEvent) => {
      this.onClose?.(ev);
      this.scheduleReconnect();
    };

    this.ws.onerror = (ev: Event) => {
      this.onError?.(ev);
    };
  }

  /** Send a chat message to the agent. */
  sendMessage(content: string): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket is not connected');
    }

    const history = this.getHistory();
    
    this.ws.send(JSON.stringify({ 
      type: 'message', 
      content,
      history,
      session_id: this.sessionId,
    }));

    // Add user message to local history
    const newHistory = [...history, { role: 'user' as const, content }];
    this.saveHistory(newHistory);
    window.dispatchEvent(new Event('zeroclaw-chat-updated'));
  }

  /** Handle incoming message - call this to track responses */
  handleMessage(msg: WsMessage): void {
    // Track assistant responses in history
    if (msg.type === 'done' && msg.full_response) {
      const history = this.getHistory();
      const newHistory = [...history, { role: 'assistant' as const, content: msg.full_response }];
      this.saveHistory(newHistory);
      
      // Dispatch event for UI to reload sessions
      window.dispatchEvent(new Event('zeroclaw-chat-updated'));
    }
    
    // Also call the user's message handler
    this.onMessage?.(msg);
  }

  /** Close the connection without auto-reconnecting. */
  disconnect(): void {
    this.intentionallyClosed = true;
    this.clearReconnectTimer();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /** Returns true if the socket is open. */
  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  // ---------------------------------------------------------------------------
  // Reconnection logic
  // ---------------------------------------------------------------------------

  private scheduleReconnect(): void {
    if (this.intentionallyClosed || !this.autoReconnect) return;

    this.reconnectTimer = setTimeout(() => {
      this.currentDelay = Math.min(this.currentDelay * 2, this.maxReconnectDelay);
      this.connect();
    }, this.currentDelay);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }
}
