// How it works on the bot page. Each step's picture plays its moment when it scrolls into view, and
// its highlighted spot can be clicked to act it out. On a first visit the page offers a guided
// walkthrough that dims the page, lights one picture at a time and moves on as the visitor clicks.
(() => {
    const how = document.getElementById('how');
    if (!how) {
        return;
    }
    const still = window.matchMedia('(prefers-reduced-motion: reduce)');
    const SEEN = 'uma-stable-match:bot-walkthrough-seen';

    let touring = false;

    const sleep = (ms) => new Promise((done) => setTimeout(done, still.matches ? 0 : ms));
    function restart(el, name) {
        el.classList.remove(name);
        void el.offsetWidth;
        el.classList.add(name);
    }
    // Adds `names` to `el` and plays its entrance.
    function enter(el, ...names) {
        el.classList.add(...names);
        restart(el, 'anim');
    }
    function press(el) {
        el.classList.add('pressed');
        setTimeout(() => el.classList.remove('pressed'), 140);
    }
    function load(loading, amount) {
        loading.classList.toggle('on', amount > 0 && amount < 2);
        if (amount <= 1) {
            loading.style.setProperty('--load', String(amount));
        }
    }

    // A scene is one picture: `reset` puts it back, `play` brings its moment in, and `act` is what
    // clicking its spot does. Each run gets a token, and stops once a reset or a newer run
    // replaces it.
    function scene(name, { reset, play, act }) {
        const vis = how.querySelector(`[data-scene="${name}"]`);
        const target = vis.querySelector('[data-target]');
        const $ = (selector) => vis.querySelector(selector);
        const $$ = (selector) => [...vis.querySelectorAll(selector)];
        const s = { vis, target, state: 'idle', token: 0 };
        const live = (token) => () => token === s.token;
        s.reset = () => {
            s.token += 1;
            s.state = 'idle';
            vis.classList.remove('done');
            target.classList.remove('beckon', 'pressed');
            for (const el of $$('.anim, .pinged')) {
                el.classList.remove('anim', 'pinged');
            }
            reset($, $$);
        };
        s.play = async () => {
            if (s.state !== 'idle') {
                return;
            }
            s.state = 'playing';
            const alive = live(s.token);
            await play($, $$, alive);
            if (alive()) {
                s.state = 'ready';
                target.classList.add('beckon');
            }
        };
        s.act = async () => {
            if (s.state === 'acting' || s.state === 'done') {
                return;
            }
            s.token += 1;
            const alive = live(s.token);
            s.state = 'acting';
            target.classList.remove('beckon');
            press(target);
            await act($, $$, alive);
            if (alive()) {
                s.state = 'done';
                vis.classList.add('done');
            }
        };
        return s;
    }

    const scenes = [
        scene('apply', {
            reset($) {
                $('[data-page="finder"]').classList.add('on');
                $('[data-page="mine"]').classList.remove('on');
                $('[data-url]').textContent = '';
                $('[data-row]').classList.remove('shown');
                load($('[data-loading]'), 0);
            },
            async play() {},
            async act($, $$, alive) {
                const loading = $('[data-loading]');
                await sleep(100);
                load(loading, 0.7);
                await sleep(450);
                if (!alive()) return;
                load(loading, 1);
                $('[data-url]').textContent = '/applications/trainer';
                $('[data-page="finder"]').classList.remove('on');
                $('[data-page="mine"]').classList.add('on');
                await sleep(150);
                if (!alive()) return;
                load(loading, 2);
                enter($('[data-row]'), 'shown');
                await sleep(200);
                load(loading, 0);
                await sleep(450);
            },
        }),
        scene('ticket', {
            reset($, $$) {
                for (const el of $$('[data-chat]')) {
                    el.classList.remove('shown');
                }
                $('[data-typing]').classList.remove('on');
                $('[data-target]').classList.remove('shown', 'unread', 'ping', 'sel');
                $('[data-general]').classList.add('sel');
                $('.dc').classList.remove('in-ticket');
            },
            async play($, $$, alive) {
                const [hello, domino, join, party, cake] = $$('[data-chat]');
                const typing = $('[data-typing]');
                const ticket = $('[data-target]');
                const say = async (who, message, ms) => {
                    if (!alive()) return;
                    $('[data-typing-name]').textContent = who;
                    typing.classList.add('on');
                    await sleep(ms);
                    typing.classList.remove('on');
                    if (alive()) enter(message, 'shown');
                    await sleep(500);
                };
                await sleep(300);
                await say('Mejiro McQueen', hello, 900);
                await say('Gold Ship', domino, 1000);
                if (!alive()) return;
                enter(join, 'shown');
                await sleep(700);
                if (!alive()) return;
                enter(ticket, 'shown', 'unread');
                await sleep(600);
                if (!alive()) return;
                ticket.classList.add('ping');
                restart(ticket, 'pinged');
                // The banter carries on, ticket or not.
                say('Mejiro McQueen', party, 1100).then(() => say('Gold Ship', cake, 1200));
            },
            async act($, $$, alive) {
                const ticket = $('[data-target]');
                for (const el of $$('[data-chat]')) {
                    el.classList.add('shown');
                }
                $('[data-typing]').classList.remove('on');
                ticket.classList.add('shown');
                await sleep(75);
                if (!alive()) return;
                $('[data-general]').classList.remove('sel');
                ticket.classList.remove('unread', 'ping');
                ticket.classList.add('sel');
                enter($('.dc'), 'in-ticket');
                await sleep(300);
            },
        }),
        scene('stats', {
            reset($, $$) {
                for (const el of $$('[data-field]')) {
                    el.classList.remove('look');
                }
            },
            async play($, $$, alive) {
                await sleep(400);
                for (const field of $$('[data-field]')) {
                    if (!alive()) return;
                    field.classList.add('look');
                    await sleep(900);
                    field.classList.remove('look');
                    await sleep(150);
                }
            },
            async act($, $$) {
                for (const el of $$('[data-field]')) {
                    el.classList.remove('look');
                }
                await sleep(200);
            },
        }),
        scene('compare', {
            reset($, $$) {
                $('.cmp').classList.remove('drawing');
                const [discord, page] = $$('.tab');
                discord.classList.remove('on');
                page.classList.add('on');
            },
            async play($) {
                await sleep(200);
                restart($('.cmp'), 'drawing');
                await sleep(2200);
            },
            async act($, $$) {
                const [discord, page] = $$('.tab');
                page.classList.remove('on');
                discord.classList.add('on');
                await sleep(200);
            },
        }),
        scene('answer', {
            reset($) {
                $('[data-actions]').classList.remove('gone');
                $('[data-msg]').classList.remove('answered');
                $('[data-mini]').classList.remove('shown');
                const status = $('[data-status]');
                status.classList.replace('accepted', 'pending');
                status.textContent = 'Pending';
                load($('[data-loading]'), 0);
            },
            async play() {},
            async act($, $$, alive) {
                await sleep(125);
                if (!alive()) return;
                $('[data-actions]').classList.add('gone');
                enter($('[data-msg]'), 'answered');
                await sleep(550);
                if (!alive()) return;
                $('[data-mini]').classList.add('shown');
                await sleep(350);
                if (!alive()) return;
                const loading = $('[data-loading]');
                load(loading, 0.8);
                await sleep(350);
                if (!alive()) return;
                load(loading, 1);
                const status = $('[data-status]');
                status.classList.replace('pending', 'accepted');
                status.textContent = 'Accepted';
                restart(status, 'anim');
                await sleep(150);
                load(loading, 2);
                await sleep(200);
                load(loading, 0);
            },
        }),
    ];

    // Looking around: a picture plays once it's mostly in view, and its spot or Replay can be
    // clicked at any time.
    // Acts out a scene for a visitor looking around, then carries them on to the next step, which
    // plays as it comes into view.
    async function explore(s) {
        await s.act();
        const after = scenes[scenes.indexOf(s) + 1];
        if (after && s.state === 'done' && !touring) {
            await sleep(175);
            bringIntoView(after.vis);
        }
    }
    const watcher = new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                const s = scenes.find((one) => one.vis === entry.target);
                if (entry.isIntersecting && !touring && s) {
                    s.play();
                }
            }
        },
        { threshold: 0.55 },
    );
    for (const s of scenes) {
        watcher.observe(s.vis);
        s.target.addEventListener('click', () => (touring ? tour.acted(s) : explore(s)));
        s.vis.querySelector('[data-replay]').addEventListener('click', () => {
            s.reset();
            s.play();
        });
    }

    // The guided walkthrough
    const STEPS = [
        {
            title: 'A trainer finds your club',
            body: 'Your club shows up in their results with a Quick Apply button. Click Quick Apply to apply as SpeedyTrainer.',
        },
        {
            title: 'Over in your Discord',
            body: 'SpeedyTrainer joins your server, and their private ticket appears under Applications with a ping for you. Click the new ticket to open it.',
        },
        {
            title: 'Review their numbers',
            body: 'The bot fills in their pace and fan history from uma.moe. To see how they’d fit, click Compare in club.',
        },
        {
            title: 'See how they’d fit',
            body: 'SpeedyTrainer is placed among your members as if they’d joined, against your fan requirement. Click the Discord tab to head back.',
        },
        {
            title: 'Make the call',
            body: 'Click Accept. The buttons go away, and SpeedyTrainer sees your answer on their applications page.',
        },
    ];
    const FINISH = {
        title: 'That’s it',
        body: 'From Quick Apply to accepted, with the numbers in front of you the whole way. Add the bot to start taking applications.',
    };

    const ask = document.getElementById('tour-ask');
    const root = document.getElementById('tour');
    const spot = root.querySelector('[data-spot]');
    const card = root.querySelector('[data-card]');
    const blocks = [...root.querySelectorAll('[data-block]')];
    const field = (name) => card.querySelector(`[data-${name}]`);
    const back = field('back');
    const next = field('next');
    const setup = field('setup');

    function place(el, values) {
        for (const [key, value] of Object.entries(values)) {
            el.style.setProperty(`--${key}`, `${value}px`);
        }
    }

    const tour = {
        index: 0,
        finished: false,
        frame: 0,
        returnFocus: null,
        open() {
            this.returnFocus = document.activeElement;
            touring = true;
            root.hidden = false;
            document.addEventListener('keydown', keys);
            this.follow();
            this.show(0);
        },
        close() {
            touring = false;
            root.hidden = true;
            cancelAnimationFrame(this.frame);
            document.removeEventListener('keydown', keys);
            remember();
            // Pictures already on screen carry on as when looking around.
            for (const s of scenes) {
                if (s.state === 'idle' && visible(s.vis)) {
                    s.play();
                }
            }
            this.returnFocus?.focus?.({ preventScroll: true });
        },
        async show(index) {
            this.index = index;
            this.finished = false;
            const s = scenes[index];
            s.reset();
            this.text(STEPS[index], `Step ${index + 1} of ${STEPS.length}`);
            bringIntoView(s.vis);
            await settled();
            // A smooth scroll can stop short, such as when the page grew meanwhile.
            if (Math.abs(scrollY - scrollFor(s.vis)) > 4 && this.index === index) {
                window.scrollTo({ top: scrollFor(s.vis), behavior: 'instant' });
            }
            if (touring && this.index === index) {
                s.play();
            }
        },
        // The visitor clicked the lit spot, or Next.
        async acted(s) {
            if (scenes[this.index] !== s || this.finished || s.state === 'acting') {
                return;
            }
            const index = this.index;
            await s.act();
            if (!touring || this.index !== index) {
                return;
            }
            if (index + 1 < STEPS.length) {
                await sleep(175);
                if (touring && this.index === index) this.show(index + 1);
            } else {
                this.finished = true;
                this.text(FINISH, 'Done');
            }
        },
        text(step, count) {
            field('count').textContent = count;
            field('title').textContent = step.title;
            field('body').textContent = step.body;
            field('dots').replaceChildren(
                ...STEPS.map((_, i) => {
                    const dot = document.createElement('span');
                    dot.classList.toggle('current', i === this.index && !this.finished);
                    return dot;
                }),
            );
            back.disabled = this.index === 0;
            back.hidden = this.finished;
            next.textContent = this.finished ? 'Done' : 'Next';
            setup.hidden = !this.finished;
            restart(card, 'anim');
            (this.finished ? setup : next).focus({ preventScroll: true });
        },
        // Keeps the light, the click shields around it and the card on the current picture while
        // the page scrolls or resizes.
        follow() {
            const frame = () => {
                const r = scenes[this.index].vis.getBoundingClientRect();
                const pad = 8;
                const box = {
                    top: r.top - pad,
                    left: r.left - pad,
                    width: r.width + pad * 2,
                    height: r.height + pad * 2,
                };
                const w = innerWidth;
                const h = innerHeight;
                const bottom = box.top + box.height;
                const right = box.left + box.width;
                place(spot, box);
                place(blocks[0], { top: 0, left: 0, width: w, height: Math.max(0, box.top) });
                place(blocks[1], {
                    top: bottom,
                    left: 0,
                    width: w,
                    height: Math.max(0, h - bottom),
                });
                place(blocks[2], {
                    top: box.top,
                    left: 0,
                    width: Math.max(0, box.left),
                    height: box.height,
                });
                place(blocks[3], {
                    top: box.top,
                    left: right,
                    width: Math.max(0, w - right),
                    height: box.height,
                });
                place(card, cardSpot(box, card.offsetWidth, card.offsetHeight, w, h));
                this.frame = requestAnimationFrame(frame);
            };
            frame();
        },
    };

    // Beside the picture where there's room, over the step's text; else below, above, or along
    // the bottom of the screen.
    function cardSpot(box, cw, ch, w, h) {
        const gap = 16;
        const middle = Math.min(Math.max(gap, box.top + box.height / 2 - ch / 2), h - ch - gap);
        const roomLeft = box.left - gap * 2;
        const roomRight = w - box.left - box.width - gap * 2;
        if (Math.max(roomLeft, roomRight) >= cw) {
            return roomLeft >= roomRight
                ? { top: middle, left: box.left - gap * 2 - cw }
                : { top: middle, left: box.left + box.width + gap * 2 };
        }
        const left = Math.max(gap, (w - cw) / 2);
        if (box.top + box.height + gap + ch <= h - gap) {
            return { top: box.top + box.height + gap, left };
        }
        if (box.top - gap - ch >= gap) {
            return { top: box.top - gap - ch, left };
        }
        return { top: h - ch - gap, left };
    }

    // Where to scroll so the picture is centred, or on narrow screens sits under the navbar so the
    // card fits below. Measured from the layout, as a picture still floating in is shifted down.
    function scrollFor(vis) {
        let top = 0;
        for (let el = vis; el; el = el.offsetParent) {
            top += el.offsetTop;
        }
        if (innerWidth < 860) {
            return top - (document.querySelector('.navbar')?.offsetHeight ?? 0) - 12;
        }
        return top - (innerHeight - vis.offsetHeight) / 2;
    }
    function bringIntoView(vis) {
        window.scrollTo({ top: scrollFor(vis), behavior: still.matches ? 'instant' : 'smooth' });
    }
    function visible(el) {
        const r = el.getBoundingClientRect();
        return r.bottom > 0 && r.top < innerHeight;
    }
    // Resolves once a smooth scroll has stopped.
    function settled() {
        return new Promise((done) => {
            let last = -1;
            let calm = 0;
            const check = () => {
                calm = scrollY === last ? calm + 1 : 0;
                last = scrollY;
                if (calm >= 4) done();
                else requestAnimationFrame(check);
            };
            requestAnimationFrame(check);
            setTimeout(done, 1500);
        });
    }
    function remember() {
        try {
            localStorage.setItem(SEEN, '1');
        } catch {
            // Without storage the offer comes back next time.
        }
    }
    function keys(event) {
        if (event.key === 'Escape') {
            event.preventDefault();
            tour.close();
        } else if (event.key === 'ArrowRight' && !tour.finished) {
            next.click();
        } else if (event.key === 'ArrowLeft' && !back.disabled && !tour.finished) {
            back.click();
        } else if (event.key === 'Tab') {
            // Focus stays in the card.
            const items = [
                ...card.querySelectorAll('button:not([hidden]):not(:disabled), a:not([hidden])'),
            ];
            const at = items.indexOf(document.activeElement);
            event.preventDefault();
            items[(at + (event.shiftKey ? -1 : 1) + items.length) % items.length]?.focus();
        }
    }

    next.addEventListener('click', () =>
        tour.finished ? tour.close() : tour.acted(scenes[tour.index]),
    );
    back.addEventListener('click', () => {
        if (tour.index > 0) tour.show(tour.index - 1);
    });
    field('skip').addEventListener('click', () => tour.close());
    setup.addEventListener('click', remember);
    const start = () => {
        ask.hidden = true;
        tour.open();
    };
    document.querySelector('[data-tour-start]').addEventListener('click', start);
    ask.querySelector('[data-tour-yes]').addEventListener('click', start);
    ask.querySelector('[data-tour-no]').addEventListener('click', () => {
        ask.hidden = true;
        remember();
    });

    let seen = false;
    try {
        seen = localStorage.getItem(SEEN) === '1';
    } catch {
        seen = false;
    }
    ask.hidden = seen;
})();
