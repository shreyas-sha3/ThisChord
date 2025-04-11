let http_url="http://localhost:8080"
let ws_url="ws://localhost:8080"
//let http_url="https://rust-chat-um86.onrender.com"
//let ws_url="wss://rust-chat-um86.onrender.com"

let socket
let username
const statusElement = document.getElementById("username");
const notificationSound = new Audio("./notify.mp3");

//PRE-CHAT
fetch(`${http_url}/auth-check`, {
    credentials: "include"
  })
  .then(res => res.json())
  .then(data => {
    if (data.status === "ok") {
    ConnectSocket()
    username = data.username;
    let usernameElement = document.getElementById("username");
    usernameElement.textContent = username;
    } else {
    window.location.href = "./auth.html";
    }
  });


//CHAT

function ToggleStatus(status){
    if(status==1){
    statusElement.textContent = username;
    //statusElement.style.color="#b8bb26";
    }
    else if(status==0){
        statusElement.textContent = "Connecting...";
        //statusElement.style.color="#83a598";
    }
    else if(status==-1){
        statusElement.textContent = "Disconnected...";
        //statusElement.style.color="#fb4934";
    }
}

let pingInterval;
function ConnectSocket() {
    ToggleStatus(0);
    socket = new WebSocket(`${ws_url}/chat`);
    socket.onopen = () => {
        socket.send(JSON.stringify({ type: "fetch_dm_list" }));
        ToggleStatus(1);
        //restoreDMs();

        pingInterval = setInterval(() => {
            fetch(`${ur}/ping`, {
                method: "GET", 
            })
            .then(response => {
                if (!response.ok) {
                    console.warn("Ping failed:", response.statusText);
                }
            })
            .catch(err => {
                console.error("Ping error:", err);
            });
        }, 10 * 60 * 1000);
    }
    socket.onclose = () => {
        ToggleStatus(-1);
        clearInterval(pingInterval); 
    };

    socket.onmessage = (event) => {
        const parsed = JSON.parse(event.data);
    
        if (parsed.type === "dm_list") {
            for (const username of parsed.users) {
                createDM(username); // your existing function
            }
            return;
        }
        if (parsed.type === "dm_history") {
            if (parsed.with === dm_recipient) {
                renderChatMessages(parsed.messages);
            }
            return;
        }
        // only destructure if it's actually an array
        if (Array.isArray(parsed)) {
            const [sender, messageText, msgType, targetUser] = parsed;
            const key = msgType === "dm"
                ? (sender === username ? targetUser : sender)
                : null;
                
            displayMessage(event.data);
    
            if (document.hidden) {
                notificationSound.play().catch((err) => {
                    console.warn("Notification sound failed to play:", err);
                });
            }
        } else {
            console.warn("Unknown message format:", parsed);
        }
    };
    
}
function sendMessage(event) {
    event.preventDefault();
    const MessageBox = document.getElementById("ClientMessage");
    const msg = MessageBox.value.trim();
    if (msg) {
        const payload = {
            type: msg_type,
            msg: msg
        };
        if (msg_type === "dm") {
            payload.to = dm_recipient;
        }
        socket.send(JSON.stringify(payload));
        MessageBox.value = "";
    }
}


let msg_type = "broadcast", dm_recipient;
dm_recipient = null;

function saveChat(username, messages) {
    const trimmed = messages.slice(-30); 
    localStorage.setItem(`chat_${username || 'public'}`, JSON.stringify(trimmed));
}

function loadChat(username) {
    return JSON.parse(localStorage.getItem(`chat_${username || 'public'}`)) || [];
}


function renderChatMessages(messages) {
    const chatBox = document.getElementById("ChatBox");
    chatBox.innerHTML = ""; // Clear existing messages
    lastUser = "";

    messages.forEach(msg => {
        if (typeof msg === "string") {
            const fallback = document.createElement("div");
            fallback.textContent = msg;
            chatBox.appendChild(fallback);
            return;
        }
        const { sender, message,timestamp  } = msg;
        console.log(msg)
        const element = createMessageElement(sender, message,timestamp);
        chatBox.appendChild(element);
    });

    chatBox.scrollTop = chatBox.scrollHeight;
}


function createMessageElement(sender, message , timestamp) {
    const messageContainer = document.createElement("div");

    if (sender !== lastUser) {
        const messageUser = document.createElement("button");
        messageUser.textContent = sender;
        messageContainer.appendChild(messageUser);
        lastUser = sender;
    }

    const messageContent = document.createElement("p");
    messageContent.innerHTML = String(message)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/\n/g, "<br>");

    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const timeOnly = `${hours}:${minutes}`;
    const timestampElement = document.createElement("p");
    timestampElement.textContent = timeOnly;
    timestampElement.classList.add("timestamp"); 

    messageContainer.appendChild(messageContent);
    messageContainer.appendChild(timestampElement);
    const isSelf = sender === username;

    messageContainer.classList.add(
        sender === "SERVER" ? "ServerMessage" :
        isSelf ? "SelfMessage" :
        "OtherMessage"
    );

    return messageContainer;
}


// function appendMessage(username, message) {
//     const messages = loadChat(username);
//     messages.push(message);
//     saveChat(username, messages);
//     renderChatMessages(messages);
// }

function showLoadingSkeleton() {
    const chatBox = document.getElementById('ChatBox');
    chatBox.innerHTML = '';
    const loadingMsg = document.createElement("p");
    loadingMsg.textContent = "Loading messages";
    loadingMsg.classList.add("LoadingMessage"); 
    chatBox.appendChild(loadingMsg);
    let dotCount = 0;
    const interval = setInterval(() => {
        dotCount = (dotCount + 1) % 4;
        loadingMsg.textContent = "Loading messages" + ".".repeat(dotCount);
    }, 500);
    //remove in 3 secs
    setTimeout(() => {
        clearInterval(interval);
        loadingMsg.remove();
    }, 3000);
}
function switchToDM(username) {
    const chatTitle = document.getElementById('ChatTitle');
    const messageInput = document.getElementById('ClientMessage');
    const messagesDiv = document.getElementById("ChatBox");

    if (dm_recipient !== username) {
        msg_type = username ? "dm" : "broadcast";
        dm_recipient = username;

        // Start fade out
        messagesDiv.classList.add("fade-transition");

        setTimeout(() => {
            if (username) {
                showLoadingSkeleton();
                socket.send(JSON.stringify({ type: "load_dm", with: username }));
            } else {
                const cachedMessages = loadChat(username);
                renderChatMessages(cachedMessages);
            }

            chatTitle.textContent = username ? `${dm_recipient}` : 'Public Chat';
            messageInput.placeholder = username ? `Message ${dm_recipient}...` : `Type a message...`;

            // Fade in
            messagesDiv.classList.remove("fade-transition");
        }, 200); // Wait for fade-out to finish
    }

    requestAnimationFrame(() => {
        messagesDiv.scrollTop = messagesDiv.scrollHeight;
    });

    // Remove notification dot if present
    const dmButtons = document.querySelectorAll(".dm-btn");
    dmButtons.forEach(btn => {
        if (btn.textContent.trim().startsWith(username)) {
            const dot = btn.querySelector(".notification-dot");
            if (dot) dot.remove();
        }
    });

    messageInput.focus();
}




// === Clicking on a chat user ===
document.getElementById("ChatBox").addEventListener("click", (event) => {
    if (event.target.tagName === "BUTTON") {
        let button_text = event.target.textContent.trim();        
        if (button_text.includes("::")) {
            button_text = button_text.split("::").pop().trim();
        }
        createDM(button_text)
        switchToDM(button_text);
    }
});


// Add user to DM list if not already present
function createDM(dmUsername) {
    const dmPane = document.getElementById("dm-pane");
    const existingDM = Array.from(dmPane.querySelectorAll(".dm-btn")).find(
        btn => btn.textContent.trim().startsWith(dmUsername)
    );

    if (!existingDM) {
        const newDM = document.createElement("button");
        newDM.className = "dm-btn";
        newDM.textContent = dmUsername;
        newDM.addEventListener("click", () => switchToDM(dmUsername));
        dmPane.appendChild(newDM);

        const dmList = JSON.parse(localStorage.getItem("dmList") || "[]");
        if (!dmList.includes(dmUsername)) {
            dmList.push(dmUsername);
            localStorage.setItem("dmList", JSON.stringify(dmList));
        }
    } else {
        if (!existingDM.querySelector(".notification-dot")) {
            const dot = document.createElement("span");
            dot.className = "notification-dot";
            dot.style.float = "right";
            dot.style.marginLeft = "auto";
            existingDM.appendChild(dot);
        }
    }
}

// function restoreDMs() {
//     const dmList = JSON.parse(localStorage.getItem("dmList") || "[]");
//     dmList.forEach(username => createDM(username));
// }
// === Server button resets to public chat ===
document.querySelector('.server-btn:first-child').addEventListener('click', () => {
    switchToDM(null);
});

//typing focuses on messagebar
document.addEventListener("keydown", function (event) {
    const input = document.getElementById("ClientMessage");

    // Ignore if already focused
    if (document.activeElement === input) return;
    if (event.key.length === 1) {
        input.focus();
        input.value += event.key;  
        event.preventDefault();    
    }
});

let lastUser=""



//FUNCTION CALLED WHEN NEW MESSAGE ARRIVES
function displayMessage(text) {
    let data;
    try {
        data = JSON.parse(text);
    } catch {
        return;
    }

    let [user, message, type, to , timestamp] = data;
    const isDM = type === "dm";
    const isSelf = user === username;

    if (isDM) {
        if (dm_recipient !== user && dm_recipient !== to) {
            console.log(`Skipping DM message not in current view: ${user} -> ${to}`);
            createDM(user);
            return;
        }
    } else if (msg_type === "dm") {
        console.log(`Skipping public message while in DM view`);
        return;
    }

    if (isDM) {
        if (dm_recipient !== user && dm_recipient !== to) {
            return;
        }
    } else if (msg_type === "dm") {
        return;
    }

    const messagesDiv = document.getElementById("ChatBox");

    if (!timestamp) timestamp = new Date().toISOString();
    const element = createMessageElement(user, message,timestamp);
    messagesDiv.appendChild(element);

    const shouldScroll = messagesDiv.scrollTop + messagesDiv.clientHeight >= messagesDiv.scrollHeight - 100;
    if (shouldScroll || isSelf) {
        requestAnimationFrame(() => {
            const messagesDiv = document.getElementById("ChatBox");
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        });
    }
    const key = isDM ? (user === username ? to : user) : null;
    const messages = loadChat(key);    
    
    messages.push({ 
        sender: user, 
        message, 
        msgType: type, 
        timestamp 
    });
    
    if (!isDM) {
        saveChat(key, messages);
    }
}

async function insertRandomEmoji() {
    const res = await fetch("https://unpkg.com/emoji.json/emoji.json");
    const emoji = (await res.json())[Math.floor(Math.random() * 6500)].char;
    const input = document.getElementById("ClientMessage");
    input.setRangeText(emoji, input.selectionStart, input.selectionEnd, "end");
    input.focus();
    document.getElementById("emoji-button").textContent = emoji;
  }


const logoutButton = document.getElementById("logout-button");
if (logoutButton) {
  logoutButton.addEventListener("click", () => {
    document.cookie = "session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;";
    window.location.href = "/auth.html"; // Redirect to login page
  })}