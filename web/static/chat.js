const form = document.getElementById('chat-form');
const input = document.getElementById('input');
const sendBtn = document.getElementById('send-btn');
const reindexBtn = document.getElementById('reindex-btn');

const panelLeft = document.getElementById('panel-left');
const panelRight = document.getElementById('panel-right');
const msgsLeft = panelLeft.querySelector('.panel-messages');
const msgsRight = panelRight.querySelector('.panel-messages');

let modelLeft = '';
let modelRight = '';

// Load model names from server
fetch('/api/config')
  .then(r => r.json())
  .then(cfg => {
    modelLeft = cfg.model_left;
    modelRight = cfg.model_right;
    panelLeft.querySelector('.model-name').textContent = modelLeft;
    panelRight.querySelector('.model-name').textContent = modelRight;
  });

// Auto-resize textarea
input.addEventListener('input', () => {
  input.style.height = 'auto';
  input.style.height = Math.min(input.scrollHeight, 150) + 'px';
});

// Submit on Enter (Shift+Enter for newline)
input.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    form.dispatchEvent(new Event('submit'));
  }
});

form.addEventListener('submit', async (e) => {
  e.preventDefault();
  const question = input.value.trim();
  if (!question) return;

  input.value = '';
  input.style.height = 'auto';
  setLoading(true);

  // Add user message to both panels
  addMessage(msgsLeft, 'user', question);
  addMessage(msgsRight, 'user', question);

  // Create assistant messages in both panels
  const leftEl = addMessage(msgsLeft, 'assistant', '');
  const rightEl = addMessage(msgsRight, 'assistant', '');

  // Run both models in parallel
  const streamLeft = streamChat(question, modelLeft, leftEl, msgsLeft);
  const streamRight = streamChat(question, modelRight, rightEl, msgsRight);

  await Promise.allSettled([streamLeft, streamRight]);

  setLoading(false);
});

async function streamChat(question, model, assistantEl, container) {
  const stepsEl = assistantEl.querySelector('.steps');
  const contentEl = assistantEl.querySelector('.content');

  try {
    const response = await fetch('/api/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ question, model }),
    });

    if (!response.ok) {
      const err = await response.json();
      contentEl.textContent = 'Blad: ' + (err.error || response.statusText);
      return;
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    let eventType = null;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split('\n');
      buffer = lines.pop();

      for (const raw of lines) {
        const line = raw.replace(/\r$/, '');
        if (line.startsWith('event: ')) {
          eventType = line.slice(7);
        } else if (line.startsWith('data: ') && eventType) {
          try {
            const data = JSON.parse(line.slice(6));
            handleEvent(eventType, data, stepsEl, contentEl, container);
          } catch (_) {}
          eventType = null;
        }
      }
    }

    if (buffer.trim()) {
      const line = buffer.replace(/\r$/, '');
      if (line.startsWith('data: ') && eventType) {
        try {
          const data = JSON.parse(line.slice(6));
          handleEvent(eventType, data, stepsEl, contentEl, container);
        } catch (_) {}
      }
    }
  } catch (err) {
    contentEl.textContent = 'Blad polaczenia: ' + err.message;
  }
}

function handleEvent(type, data, stepsEl, contentEl, container) {
  switch (type) {
    case 'thinking': {
      const step = createStep('Mysle...', true);
      stepsEl.appendChild(step);
      break;
    }
    case 'tool_call': {
      markLastStepDone(stepsEl);
      const label = formatToolCall(data.tool, data.args);
      const step = createStep(label, true);
      stepsEl.appendChild(step);
      break;
    }
    case 'tool_result': {
      markLastStepDone(stepsEl, data.status === 'error');
      break;
    }
    case 'answer': {
      markLastStepDone(stepsEl);
      if (typeof marked !== 'undefined') {
        contentEl.innerHTML = marked.parse(data.text);
      } else {
        contentEl.textContent = data.text;
      }
      break;
    }
    case 'error': {
      markLastStepDone(stepsEl, true);
      contentEl.textContent = 'Blad: ' + data.text;
      break;
    }
  }
  scrollToBottom(container);
}

function formatToolCall(tool, args) {
  switch (tool) {
    case 'search': {
      const queries = args.query || [];
      return 'Szukam: ' + queries.map(q => '"' + q + '"').join(', ');
    }
    case 'grep':
      return 'Grep: ' + (args.pattern || '');
    case 'read_file':
      return 'Czytam: ' + (args.path || '');
    case 'glob':
      return 'Glob: ' + (args.pattern || '');
    default:
      return tool + '(...)';
  }
}

function createStep(label, spinning) {
  const el = document.createElement('div');
  el.className = 'step';
  el.innerHTML = '<span class="icon">' +
    (spinning ? '<span class="spinner"></span>' : '') +
    '</span><span class="label">' + escapeHtml(label) + '</span>';
  return el;
}

function markLastStepDone(stepsEl, isError) {
  const steps = stepsEl.querySelectorAll('.step:not(.done):not(.error)');
  if (steps.length === 0) return;
  const last = steps[steps.length - 1];
  last.classList.add(isError ? 'error' : 'done');
  last.querySelector('.icon').innerHTML = isError ? '&#x2717;' : '&#x2713;';
}

function addMessage(container, role, text) {
  const el = document.createElement('div');
  el.className = 'message ' + role;
  if (role === 'user') {
    el.textContent = text;
  } else {
    el.innerHTML = '<div class="steps"></div><div class="content"></div>';
  }
  container.appendChild(el);
  scrollToBottom(container);
  return el;
}

function setLoading(loading) {
  sendBtn.disabled = loading;
  input.disabled = loading;
  if (!loading) input.focus();
}

function scrollToBottom(container) {
  container.scrollTop = container.scrollHeight;
}

function escapeHtml(str) {
  const d = document.createElement('div');
  d.textContent = str;
  return d.innerHTML;
}

// Reindex
reindexBtn.addEventListener('click', async () => {
  reindexBtn.disabled = true;
  reindexBtn.textContent = '...';
  try {
    const resp = await fetch('/api/reindex', { method: 'POST' });
    const data = await resp.json();
    alert(data.success ? 'Reindeksacja zakonczona.' : 'Blad: ' + data.output);
  } catch (e) {
    alert('Blad: ' + e.message);
  }
  reindexBtn.disabled = false;
  reindexBtn.textContent = '\u21bb';
});
