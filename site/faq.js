// FAQ search: filters questions as you type and highlights the match.
const input = document.querySelector("#faq-q");
const list = document.querySelector(".qa");
const none = document.querySelector("#faq-none");

if (input && list) {
  const pairs = [...list.querySelectorAll("dt")].map(dt => ({ dt, dd: dt.nextElementSibling, q: dt.textContent }));
  const escape = s => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

  input.addEventListener("input", () => {
    const term = input.value.trim();
    const re = term ? new RegExp(`(${escape(term)})`, "i") : null;
    let shown = 0;
    for (const p of pairs) {
      const hit = !re || re.test(p.q) || re.test(p.dd.textContent);
      p.dt.hidden = p.dd.hidden = !hit;
      if (hit) shown++;
      // Highlight in the question only; answers keep their links intact.
      p.dt.replaceChildren();
      if (re && hit) {
        for (const part of p.q.split(new RegExp(`(${escape(term)})`, "gi"))) {
          if (part.toLowerCase() === term.toLowerCase()) {
            const m = document.createElement("mark");
            m.textContent = part;
            p.dt.append(m);
          } else if (part) {
            p.dt.append(part);
          }
        }
      } else {
        p.dt.textContent = p.q;
      }
    }
    none.hidden = shown > 0;
  });

  // /faq?q=… (the contact page's quick answers) opens already filtered.
  const q = new URLSearchParams(location.search).get("q");
  if (q) {
    input.value = q;
    input.dispatchEvent(new Event("input"));
  }
}
