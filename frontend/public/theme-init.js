try {
  const theme = localStorage.getItem('uma-stable-match:theme');
  if (theme === 'light' || theme === 'dark') {
    document.documentElement.dataset.theme = theme;
  }
} catch {
  document.documentElement.removeAttribute('data-theme');
}
