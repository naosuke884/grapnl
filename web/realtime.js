// 影の部屋との通信。WebSocket で部屋に繋ぎ、wasm とバイト列をやり取りする関数を渡す。
// mq_js_bundle.js の後、load() の前に読み込む。
// 繋がらないときは何もしない。ゲームは影なしで続く。
(function () {
    var API_ORIGIN = "https://api.roommate.hayashi-naoki.com";
    // API が受け付けるのは配信用ドメインからだけ。ほかで開いたとき（手元の確認など）は繋がない。
    var PLAY_ORIGIN = "https://play.roommate.hayashi-naoki.com";
    // 届いたメッセージを wasm が取り出すまで溜める。タブが裏にある間に溜まりすぎないよう上限を置く。
    var MAX_QUEUE = 64;
    // 繋がらないときは待ち時間を倍にしていく。繋がったら最初に戻す。
    var RECONNECT_MS = 5000;
    // 同じ ID の新しい接続に席を譲ったときの close code(roommate の apps/api/src/room.ts)。
    var REPLACED_CLOSE_CODE = 4000;
    var MAX_RECONNECT_MS = 60000;
    var reconnectDelay = RECONNECT_MS;
    // いまこのゲームを遊んでいる人数（サイトのトップやゲームページと同じ数）。部屋に入ったら読み始め、
    // サイトと同じ間隔で読み直す。まだ読めていないとき、読めないときは -1 のまま。
    var PRESENCE_URL = API_ORIGIN + "/games/grapnl/presence";
    var PRESENCE_REFRESH_MS = 30000;
    var playing = -1;
    var presenceTimer = null;

    var socket = null;
    var roomKey = null;
    var queue = [];
    var reconnectTimer = null;
    // 匿名 ID のトークンは roommate-sdk が受け持つ。roommate-sdk.js をこれより前に読み込む。
    function getToken() {
        var client = window.roommate && window.roommate.client;
        if (!client) return Promise.reject(new Error("roommate-sdk が無い"));
        return client.token().then(function (token) {
            if (!token) throw new Error("トークンが取れない");
            return token;
        });
    }

    function push(message) {
        queue.push(message);
        if (queue.length > MAX_QUEUE) queue.shift();
    }

    // 表示名は `[2][ID: u16][UTF-8 の名前]` にして wasm へ渡す。名前の中身は wasm が確かめる。
    var MAX_NAME_BYTES = 128;
    function pushName(id, name) {
        if (!Number.isInteger(id) || id < 0 || id > 0xffff || typeof name !== "string") return;
        var bytes = new TextEncoder().encode(name);
        if (bytes.length === 0 || bytes.length > MAX_NAME_BYTES) return;
        var message = new Uint8Array(bytes.length + 3);
        message[0] = 2;
        message[1] = (id >> 8) & 255;
        message[2] = id & 255;
        message.set(bytes, 3);
        push(message);
    }

    function scheduleReconnect(key) {
        if (reconnectTimer !== null) return;
        reconnectTimer = setTimeout(function () {
            reconnectTimer = null;
            if (roomKey === key) connect(key);
        }, reconnectDelay);
        reconnectDelay = Math.min(reconnectDelay * 2, MAX_RECONNECT_MS);
    }

    function connect(key) {
        getToken()
            .then(function (token) {
                if (roomKey !== key) return;
                var url =
                    API_ORIGIN.replace(/^http/, "ws") +
                    "/rooms/" +
                    encodeURIComponent(key) +
                    "?token=" +
                    encodeURIComponent(token);
                var ws = new WebSocket(url);
                ws.binaryType = "arraybuffer";
                ws.onopen = function () {
                    reconnectDelay = RECONNECT_MS;
                };
                ws.onmessage = function (event) {
                    if (socket !== ws) return;
                    if (typeof event.data === "string") {
                        // 部屋を出た人と、表示名を伝える。来た人は状態が届いたときに分かる。
                        var notice;
                        try {
                            notice = JSON.parse(event.data);
                        } catch (e) {
                            return;
                        }
                        if (!notice) return;
                        if (notice.type === "leave" && Number.isInteger(notice.id)) {
                            push(new Uint8Array([1, (notice.id >> 8) & 255, notice.id & 255]));
                        } else if (notice.type === "join") {
                            pushName(notice.id, notice.name);
                        } else if (notice.type === "welcome" && notice.names) {
                            // 先にいた人の表示名。キーは ID の文字列。
                            Object.keys(notice.names).forEach(function (id) {
                                pushName(Number(id), notice.names[id]);
                            });
                        }
                        return;
                    }
                    var data = new Uint8Array(event.data);
                    var message = new Uint8Array(data.length + 1);
                    message[0] = 0;
                    message.set(data, 1);
                    push(message);
                };
                ws.onclose = function (event) {
                    if (socket !== ws) return;
                    socket = null;
                    // 同じ ID で別のタブなどが同じ部屋に入ると、サーバーは前の接続を 4000 で閉じる。
                    // 入り直すと今度は向こうを閉じ、交互に追い出し合うので、このタブは入り直さない。
                    if (event.code === REPLACED_CLOSE_CODE) return;
                    scheduleReconnect(key);
                };
                socket = ws;
            })
            .catch(function () {
                scheduleReconnect(key);
            });
    }

    function loadPresence() {
        // タブが裏にある間は読まない。戻ってから次に読むまでは前の数を出す。
        if (document.hidden) return;
        fetch(PRESENCE_URL, { headers: { accept: "application/json" } })
            .then(function (res) {
                if (!res.ok) throw new Error("人数が読めない");
                return res.json();
            })
            .then(function (body) {
                if (body && Number.isInteger(body.playing) && body.playing >= 0) playing = body.playing;
            })
            .catch(function () {
                // 読めなければ前の数のまま。
            });
    }

    function watchPresence() {
        if (presenceTimer !== null) return;
        loadPresence();
        presenceTimer = setInterval(loadPresence, PRESENCE_REFRESH_MS);
    }

    function join(key) {
        if (window.location.origin !== PLAY_ORIGIN) return;
        watchPresence();
        if (key === roomKey) return;
        roomKey = key;
        queue = [];
        if (socket) {
            var old = socket;
            socket = null;
            old.close();
        }
        connect(key);
    }

    function register_plugin(importObject) {
        importObject.env.realtime_join = function (ptr, len) {
            join(UTF8ToString(ptr, len));
        };
        importObject.env.realtime_send = function (ptr, len) {
            if (!socket || socket.readyState !== WebSocket.OPEN) return;
            socket.send(new Uint8Array(wasm_memory.buffer, ptr, len).slice());
        };
        importObject.env.realtime_next_len = function () {
            return queue.length > 0 ? queue[0].length : -1;
        };
        importObject.env.realtime_playing = function () {
            return playing;
        };
        importObject.env.realtime_take = function (ptr, len) {
            var message = queue.shift();
            if (!message) return;
            new Uint8Array(wasm_memory.buffer, ptr, len).set(message.subarray(0, len));
        };
    }

    miniquad_add_plugin({ register_plugin: register_plugin, name: "browser_game_realtime", version: 1 });
})();
