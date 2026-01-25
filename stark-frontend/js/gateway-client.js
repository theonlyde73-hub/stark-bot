/**
 * Gateway WebSocket client for real-time communication with StarkBot Gateway
 */
class GatewayClient {
    constructor(url) {
        this.url = url;
        this.ws = null;
        this.handlers = new Map();
        this.eventListeners = new Map();
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 1000;
        this.connected = false;
        this.onConnectionChange = null;
    }

    /**
     * Connect to the Gateway WebSocket server
     */
    connect() {
        return new Promise((resolve, reject) => {
            try {
                this.ws = new WebSocket(this.url);

                this.ws.onopen = () => {
                    console.log('Gateway connected');
                    this.connected = true;
                    this.reconnectAttempts = 0;
                    if (this.onConnectionChange) {
                        this.onConnectionChange(true);
                    }
                    resolve();
                };

                this.ws.onclose = () => {
                    console.log('Gateway disconnected');
                    this.connected = false;
                    if (this.onConnectionChange) {
                        this.onConnectionChange(false);
                    }
                    this.attemptReconnect();
                };

                this.ws.onerror = (error) => {
                    console.error('Gateway error:', error);
                    reject(error);
                };

                this.ws.onmessage = (event) => {
                    this.handleMessage(event.data);
                };
            } catch (error) {
                reject(error);
            }
        });
    }

    /**
     * Attempt to reconnect to the Gateway
     */
    attemptReconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.log('Max reconnect attempts reached');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

        console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

        setTimeout(() => {
            this.connect().catch(console.error);
        }, delay);
    }

    /**
     * Handle incoming WebSocket messages
     */
    handleMessage(data) {
        try {
            const message = JSON.parse(data);

            // Check if it's an event (server push)
            if (message.type === 'event') {
                this.emitEvent(message.event, message.data);
                return;
            }

            // Check if it's a response to a pending request
            if (message.id && this.handlers.has(message.id)) {
                const { resolve, reject } = this.handlers.get(message.id);
                this.handlers.delete(message.id);

                if (message.error) {
                    reject(new Error(message.error.message));
                } else {
                    resolve(message.result);
                }
            }
        } catch (error) {
            console.error('Failed to parse Gateway message:', error);
        }
    }

    /**
     * Emit an event to registered listeners
     */
    emitEvent(event, data) {
        const listeners = this.eventListeners.get(event) || [];
        listeners.forEach(callback => callback(data));

        // Also emit to wildcard listeners
        const wildcardListeners = this.eventListeners.get('*') || [];
        wildcardListeners.forEach(callback => callback(event, data));
    }

    /**
     * Call a Gateway RPC method
     */
    async call(method, params = {}) {
        if (!this.connected) {
            throw new Error('Not connected to Gateway');
        }

        const id = crypto.randomUUID();

        return new Promise((resolve, reject) => {
            // Set a timeout for the request
            const timeout = setTimeout(() => {
                this.handlers.delete(id);
                reject(new Error('Request timeout'));
            }, 30000);

            this.handlers.set(id, {
                resolve: (result) => {
                    clearTimeout(timeout);
                    resolve(result);
                },
                reject: (error) => {
                    clearTimeout(timeout);
                    reject(error);
                }
            });

            this.ws.send(JSON.stringify({ id, method, params }));
        });
    }

    /**
     * Subscribe to Gateway events
     */
    on(event, callback) {
        if (!this.eventListeners.has(event)) {
            this.eventListeners.set(event, []);
        }
        this.eventListeners.get(event).push(callback);
    }

    /**
     * Unsubscribe from Gateway events
     */
    off(event, callback) {
        if (this.eventListeners.has(event)) {
            const listeners = this.eventListeners.get(event);
            const index = listeners.indexOf(callback);
            if (index > -1) {
                listeners.splice(index, 1);
            }
        }
    }

    /**
     * Disconnect from the Gateway
     */
    disconnect() {
        if (this.ws) {
            this.maxReconnectAttempts = 0; // Prevent reconnection
            this.ws.close();
        }
    }

    /**
     * Check if connected
     */
    isConnected() {
        return this.connected;
    }
}

// Export for use in other scripts
window.GatewayClient = GatewayClient;
