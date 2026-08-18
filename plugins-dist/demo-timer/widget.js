// HomeDesktop 插件 v2 示例：倒计时小组件（自定义元素）
// 通过 window.__homedesktopPlugin 读写实例设置（编辑模式 ⚙ 菜单）
// 通知能力：倒计时到点 → bridge.notify() → 宿主 app_notify → 系统右下角 toast
(() => {
  const TAG = "hd-timer-widget";

  class HdTimerWidget extends HTMLElement {
    connectedCallback() {
      this._start = Date.now();
      this._notified = false;
      this.timer = setInterval(() => this.render(), 1000);
      this.render();
    }

    disconnectedCallback() {
      clearInterval(this.timer);
    }

    get cellId() {
      return this.getAttribute("cell-id") || "";
    }

    async render() {
      const bridge = window.__homedesktopPlugin;
      const target = Math.max(1, Number(await bridge.getSetting(this.cellId, "target", 60)) || 60);
      // 目标秒数变化 → 重新开始倒计时（并重置“已通知”标记）
      if (target !== this._lastTarget) {
        this._lastTarget = target;
        this._start = Date.now();
        this._notified = false;
      }
      const elapsed = Math.floor((Date.now() - this._start) / 1000);
      const remain = Math.max(0, target - elapsed);
      const mm = String(Math.floor(remain / 60)).padStart(2, "0");
      const ss = String(remain % 60).padStart(2, "0");
      const pct = Math.min(100, Math.round((elapsed / target) * 100));

      if (!this.shadow) {
        this.shadow = this.attachShadow({ mode: "open" });
        this.shadow.innerHTML = `
          <style>
            :host { display: block; height: 100%; }
            .wrap {
              height: 100%;
              display: flex; flex-direction: column;
              align-items: center; justify-content: center; gap: 6px;
            }
            .time { font-size: 34px; font-weight: 700; font-variant-numeric: tabular-nums; }
            .time.done { font-size: 20px; color: #e53935; }
            .bar { width: 80%; height: 5px; background: rgba(128,128,128,.25); border-radius: 3px; overflow: hidden; }
            .fill { height: 100%; background: #1e88e5; border-radius: 3px; transition: width .6s; }
            .hint { font-size: 11px; opacity: .6; }
          </style>
          <div class="wrap">
            <div class="time"></div>
            <div class="bar"><div class="fill"></div></div>
            <div class="hint"></div>
          </div>`;
      }

      const t = this.shadow.querySelector(".time");
      const f = this.shadow.querySelector(".fill");
      const h = this.shadow.querySelector(".hint");
      if (remain <= 0) {
        t.textContent = "⏰ 时间到";
        t.classList.add("done");
        f.style.width = "100%";
        h.textContent = `目标 ${target} 秒`;
        // 通知能力：到点只发一次（避免每秒重复弹）
        if (!this._notified) {
          this._notified = true;
          const enable = (await bridge.getSetting(this.cellId, "enableNotify", true)) !== false;
          if (enable) {
            bridge
              .notify("倒计时结束", `⏱️ 目标 ${target} 秒时间到，请查看任务！`)
              .catch(() => {});
          }
        }
      } else {
        t.textContent = `${mm}:${ss}`;
        t.classList.remove("done");
        f.style.width = pct + "%";
        h.textContent = `剩余 ${remain} 秒 / 共 ${target} 秒`;
      }
    }
  }

  if (!customElements.get(TAG)) {
    customElements.define(TAG, HdTimerWidget);
  }
})();
