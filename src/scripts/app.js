const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const state = {
  files: [],
  selectedIndex: -1,
  activeTab: "split",
  lastOutputPath: ""
};

const $ = (id) => document.getElementById(id);

function log(message) {
  const panel = $("logPanel");
  const time = new Date().toLocaleTimeString();
  panel.textContent += `[${time}] ${message}\n`;
  panel.scrollTop = panel.scrollHeight;
}

function setProgress(value) {
  $("progressBar").value = Math.max(0, Math.min(100, value));
}

function renderFiles() {
  const list = $("fileList");
  list.innerHTML = "";
  state.files.forEach((item, index) => {
    const li = document.createElement("li");
    li.className = `${item.kind === "blank" ? "blank" : ""} ${index === state.selectedIndex ? "selected" : ""}`;
    li.textContent = item.kind === "blank" ? "空白页 A4" : item.path;
    li.addEventListener("click", () => {
      state.selectedIndex = index;
      renderFiles();
    });
    list.appendChild(li);
  });
}

function addPdfFiles(paths) {
  for (const path of paths) {
    if (!path || !path.toLowerCase().endsWith(".pdf")) continue;
    if (!state.files.some((item) => item.kind === "pdf" && item.path === path)) {
      state.files.push({ kind: "pdf", path });
    }
  }
  if (state.selectedIndex < 0 && state.files.length > 0) state.selectedIndex = 0;
  renderFiles();
}

function firstPdfPath() {
  const item = state.files.find((entry) => entry.kind === "pdf");
  if (!item) throw new Error("请先选择至少一个 PDF 文件");
  return item.path;
}

function requireValue(id, label) {
  const value = $(id).value.trim();
  if (!value) throw new Error(`请设置${label}`);
  return value;
}

async function runTask(label, command, options) {
  setProgress(0);
  log(`${label}开始`);
  try {
    const result = await invoke(command, { options });
    if (result.paths && result.paths.length > 0) {
      state.lastOutputPath = result.paths[0];
    }
    log(`${label}完成：${result.paths.join(", ")}`);
  } catch (error) {
    log(`${label}失败：${error}`);
  }
}

async function setupTauriEvents() {
  await listen("task-progress", (event) => {
    const data = event.payload;
    const percent = data.total ? Math.round((data.current * 100) / data.total) : 0;
    setProgress(percent);
    log(`${data.task} ${percent}% ${data.message}`);
  });
  await listen("task-complete", (event) => {
    log(event.payload.message);
  });
}

function setupTabs() {
  document.querySelectorAll(".tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      state.activeTab = tab.dataset.tab;
      document.querySelectorAll(".tab").forEach((item) => item.classList.remove("active"));
      document.querySelectorAll(".tool-form").forEach((item) => item.classList.remove("active"));
      tab.classList.add("active");
      $(`${state.activeTab}Panel`).classList.add("active");
    });
  });
}

function setupDragDrop() {
  const dropZone = $("dropZone");
  window.addEventListener("dragover", (event) => {
    event.preventDefault();
    dropZone.classList.add("dragging");
  });
  window.addEventListener("dragleave", () => dropZone.classList.remove("dragging"));
  window.addEventListener("drop", (event) => {
    event.preventDefault();
    dropZone.classList.remove("dragging");
    const paths = Array.from(event.dataTransfer.files)
      .map((file) => file.path || file.name)
      .filter(Boolean);
    addPdfFiles(paths);
  });
}

function setupButtons() {
  $("themeToggle").addEventListener("click", () => document.body.classList.toggle("dark"));
  $("pickFiles").addEventListener("click", async () => addPdfFiles(await invoke("pick_pdf_files")));
  $("clearFiles").addEventListener("click", () => {
    state.files = [];
    state.selectedIndex = -1;
    renderFiles();
  });
  $("moveUp").addEventListener("click", () => moveSelected(-1));
  $("moveDown").addEventListener("click", () => moveSelected(1));
  $("removeFile").addEventListener("click", () => {
    if (state.selectedIndex >= 0) {
      state.files.splice(state.selectedIndex, 1);
      state.selectedIndex = Math.min(state.selectedIndex, state.files.length - 1);
      renderFiles();
    }
  });
  $("addBlank").addEventListener("click", () => {
    const item = { kind: "blank", width: 595, height: 842 };
    if (state.selectedIndex >= 0) state.files.splice(state.selectedIndex + 1, 0, item);
    else state.files.push(item);
    renderFiles();
  });
  $("openOutput").addEventListener("click", async () => {
    if (!state.lastOutputPath) return log("暂无可打开的输出路径");
    await invoke("open_path", { path: state.lastOutputPath });
  });

  document.querySelectorAll("[data-pick-dir]").forEach((button) => {
    button.addEventListener("click", async () => {
      const value = await invoke("pick_output_dir");
      if (value) $(button.dataset.pickDir).value = value;
    });
  });

  document.querySelectorAll("[data-pick-file]").forEach((button) => {
    button.addEventListener("click", async () => {
      const value = await invoke("pick_output_file", { defaultName: button.dataset.default });
      if (value) $(button.dataset.pickFile).value = value;
    });
  });
}

function moveSelected(offset) {
  const from = state.selectedIndex;
  const to = from + offset;
  if (from < 0 || to < 0 || to >= state.files.length) return;
  const [item] = state.files.splice(from, 1);
  state.files.splice(to, 0, item);
  state.selectedIndex = to;
  renderFiles();
}

function setupForms() {
  $("splitPanel").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      const mode = $("splitMode").value;
      const value = requireValue("splitValue", "页码/数量");
      const type = mode === "range" ? "ranges" : mode === "every" ? "every" : "single";
      const modePayload =
        type === "ranges"
          ? { type, ranges: value }
          : type === "every"
            ? { type, pagesPerFile: Number(value) }
            : { type, page: Number(value) };
      await runTask("PDF分割", "split_pdf_task", {
        input: firstPdfPath(),
        outputDir: requireValue("splitOutput", "输出目录"),
        mode: modePayload
      });
    } catch (error) {
      log(error.message || String(error));
    }
  });

  $("mergePanel").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      const items = state.files.map((item) =>
        item.kind === "blank"
          ? { kind: "blank", width: item.width, height: item.height }
          : { kind: "pdf", path: item.path }
      );
      await runTask("PDF合并", "merge_pdf_task", {
        items,
        output: requireValue("mergeOutput", "输出 PDF")
      });
    } catch (error) {
      log(error.message || String(error));
    }
  });

  $("textPanel").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      await runTask("文本提取", "text_pdf_task", {
        input: firstPdfPath(),
        output: requireValue("textOutput", "输出 TXT"),
        withPageMarkers: $("pageMarkers").checked
      });
    } catch (error) {
      log(error.message || String(error));
    }
  });

  $("imgPanel").addEventListener("submit", async (event) => {
    event.preventDefault();
    try {
      await runTask("PDF转图片", "image_pdf_task", {
        input: firstPdfPath(),
        outputDir: requireValue("imgOutput", "输出目录"),
        dpi: Number($("imgDpi").value),
        format: $("imgFormat").value,
        pages: $("imgPages").value.trim() || null
      });
    } catch (error) {
      log(error.message || String(error));
    }
  });
}

setupTauriEvents();
setupTabs();
setupDragDrop();
setupButtons();
setupForms();
renderFiles();
log("就绪");

