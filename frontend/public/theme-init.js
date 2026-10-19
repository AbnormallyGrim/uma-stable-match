// Applies a saved theme before the page renders. It is its own file so the
// content security policy needs no inline script.
try {
  const theme = localStorage.getItem('uma-stable-match:theme');
  if (theme === 'light' || theme === 'dark') {
    document.documentElement.dataset.theme = theme;
  }
} catch {
  document.documentElement.removeAttribute('data-theme');
}
