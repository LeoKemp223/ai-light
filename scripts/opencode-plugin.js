/**
 * AI Light OpenCode Plugin
 *
 * Sends OpenCode session events to AI Light's local HTTP server.
 *
 * ## Installation
 *
 * Add to ~/.config/opencode/opencode.json:
 * {
 *   "plugins": {
 *     "https://raw.githubusercontent.com/lcy05/ai-light/main/scripts/opencode-plugin.js": {}
 *   }
 * }
 *
 * Or copy this file to ~/.config/opencode/plugins/ai-light.js
 */

/// <reference types="@opencode-ai/plugin" />

/**
 * @type {import("@opencode-ai/plugin").Plugin}
 */
export default async function aiLightPlugin(ctx) {
  const AI_LIGHT_URL =
    process.env.AI_LIGHT_URL ||
    "http://127.0.0.1:17321";

  function baseUrl() {
    return AI_LIGHT_URL.replace(/\/+$/, "");
  }

  async function sendEvent(eventType, payload) {
    try {
      const url = `${baseUrl()}/events`;
      const body = JSON.stringify({
        event_type: eventType,
        session_id: payload.session?.id || "unknown",
        sessionId: payload.session?.id || "unknown",
        cwd: payload.session?.cwd || payload.cwd || process.cwd(),
        tool_call: payload.tool?.name || payload.toolName || null,
        toolName: payload.tool?.name || payload.toolName || null,
        tool_source: "opencode",
        source: "opencode",
      });
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body,
      });
      if (!response.ok) {
        console.error(`AI Light: event ${eventType} failed (${response.status})`);
      }
    } catch (error) {
      // AI Light not running — silently ignore
    }
  }

  return {
    "session.created": async (input) => {
      await sendEvent("session-start", input);
    },
    "session.deleted": async (input) => {
      await sendEvent("session-end", input);
    },
    "session.idle": async (input) => {
      await sendEvent("stop", input);
    },
    "tool.execute.before": async (input) => {
      await sendEvent("pre-tool-use", input);
    },
    "tool.execute.after": async (input) => {
      await sendEvent("post-tool-use", input);
    },
    "message.updated": async (input) => {
      if (input.role === "user") {
        await sendEvent("prompt-submit", input);
      }
    },
  };
}
