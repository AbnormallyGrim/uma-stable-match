// Floats elements up as they scroll into view. The head loads this first, so `js` is set before
// the page paints and the elements start hidden only when this script is running.
document.documentElement.classList.add('js');

document.addEventListener('DOMContentLoaded', () => {
    const items = document.querySelectorAll('.reveal');
    if (!('IntersectionObserver' in window)) {
        for (const el of items) {
            el.classList.add('in');
        }
        return;
    }
    const observer = new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                if (entry.isIntersecting) {
                    entry.target.classList.add('in');
                    observer.unobserve(entry.target);
                }
            }
        },
        { threshold: 0.15 },
    );
    for (const el of items) {
        observer.observe(el);
    }
});
