// Home page: the four-scene demo in the hero, and the download hint.
// The progress bar under the active step is a CSS animation; when it ends,
// the next scene plays. Hover or keyboard focus pauses; clicking jumps.

const demo = document.querySelector(".demo");
if (demo) {
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const steps = [...demo.querySelectorAll(".demo-steps button")];
  const scenes = [...demo.querySelectorAll(".scene")];
  let index = 0;

  // Numbers count up with an ease-out curve when their scene appears.
  const countUp = (el, to, ms) => {
    if (reduce) {
      el.textContent = to.toLocaleString("en-US");
      return;
    }
    const start = performance.now();
    const tick = now => {
      const t = Math.min(1, (now - start) / ms);
      el.textContent = Math.round(to * (1 - Math.pow(1 - t, 3))).toLocaleString("en-US");
      if (t < 1 && scenes[index].contains(el)) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  };

  // Scene 2: file names stream past while the bar fills.
  const names = [
    "Documents/Taxes 2025.pdf",
    "Pictures/Lisbon trip/IMG_4471.jpg",
    "Desktop/thesis-final-v7.docx",
    "Music/Night Drive.flac",
    "Documents/Lease — signed.pdf",
    "Pictures/Mum 60th/IMG_0192.heic",
  ];
  let ticker = 0;
  const streamNames = () => {
    clearInterval(ticker);
    const el = demo.querySelector(".ticker");
    if (!el) return;
    let i = 0;
    el.textContent = names[0];
    if (reduce) return;
    ticker = setInterval(() => (el.textContent = names[++i % names.length]), 520);
  };

  const show = (i, fromUser = false) => {
    index = i;
    steps.forEach((b, n) => {
      b.setAttribute("aria-selected", String(n === i));
      b.tabIndex = n === i ? 0 : -1;
      b.classList.toggle("seen", n < i);
      // Restart the progress animation on the active step.
      const bar = b.querySelector("i");
      bar.style.animation = "none";
      void bar.offsetWidth;
      bar.style.animation = "";
    });
    scenes.forEach((s, n) => s.classList.toggle("on", n === i));
    for (const el of scenes[i].querySelectorAll("[data-count]")) countUp(el, Number(el.dataset.count), Number(el.dataset.ms) || 1600);
    if (scenes[i].querySelector(".ticker")) streamNames();
    else clearInterval(ticker);
    if (fromUser) demo.classList.remove("paused");
  };

  steps.forEach((b, n) => {
    b.addEventListener("click", () => show(n, true));
    b.addEventListener("keydown", e => {
      if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
      const next = (n + (e.key === "ArrowRight" ? 1 : steps.length - 1)) % steps.length;
      show(next, true);
      steps[next].focus();
    });
    b.querySelector("i").addEventListener("animationend", () => {
      if (n === index) show((index + 1) % steps.length);
    });
  });

  if (reduce) demo.classList.add("still");
  demo.addEventListener("pointerenter", () => demo.classList.add("paused"));
  demo.addEventListener("pointerleave", () => demo.classList.remove("paused"));
  demo.addEventListener("focusin", () => demo.classList.add("paused"));
  demo.addEventListener("focusout", () => demo.classList.remove("paused"));
  // Don't burn cycles (or skip scenes) while the tab is hidden.
  document.addEventListener("visibilitychange", () => demo.classList.toggle("paused", document.hidden));

  show(0);
}

// ── Download hint: is this the right computer? ───────────────────────────────
const hint = document.querySelector(".os-hint");
if (hint) {
  const ua = navigator.userAgent;
  // userAgentData.platform can be "" — fall through to the UA string then.
  const platform = navigator.userAgentData?.platform || navigator.platform || ua;
  const mobile = /Android|iPhone|iPad|Mobile/i.test(ua);
  if (mobile) {
    hint.textContent = "You're on a phone or tablet. Download Ferry on the Windows computer you're moving away from.";
  } else if (/Win/i.test(platform)) {
    hint.textContent = "✓ You're on Windows — this is the right download.";
    hint.classList.add("good");
  } else if (/Mac|Linux/i.test(platform)) {
    const os = /Mac/i.test(platform) ? "a Mac" : "Linux";
    hint.textContent = `You're on ${os}. Ferry runs on the Windows computer you're moving away from — download it there.`;
  }
  if (hint.textContent) hint.classList.remove("hidden");
}
