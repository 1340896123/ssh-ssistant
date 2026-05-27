import http from "node:http";

const port = Number(process.env.MOCK_OPENAI_PORT || 5059);

const server = http.createServer(async (req, res) => {
  if (req.method !== "POST" || req.url !== "/chat/completions") {
    res.writeHead(404, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: "not found" }));
    return;
  }

  let body = "";
  for await (const chunk of req) {
    body += chunk;
  }

  let parsed = {};
  try {
    parsed = JSON.parse(body || "{}");
  } catch {
    parsed = {};
  }

  const model = parsed.model || "mock-model";
  const userMessage = Array.isArray(parsed.messages)
    ? parsed.messages.find((message) => message.role === "user")?.content || ""
    : "";

  const payload = {
    id: "chatcmpl-mock",
    object: "chat.completion",
    created: Math.floor(Date.now() / 1000),
    model,
    choices: [
      {
        index: 0,
        finish_reason: "stop",
        message: {
          role: "assistant",
          content: `mock response: ${String(userMessage).slice(0, 80)}`,
        },
      },
    ],
    usage: {
      prompt_tokens: 12,
      completion_tokens: 8,
      total_tokens: 20,
    },
  };

  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify(payload));
});

server.listen(port, "127.0.0.1", () => {
  console.log(JSON.stringify({ ok: true, port }));
});
