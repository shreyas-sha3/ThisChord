
//let http_url="http://localhost:8080"
//let ws_url="ws://localhost:8080"
let http_url="https://rust-chat-um86.onrender.com"
let ws_url="wss://rust-chat-um86.onrender.com"

let socket
let username; 
const statusElement = document.getElementById("username");
const notificationSound = new Audio("./assets/notify.mp3");
setVh(); // Initial run
init();

async function init() {
  try {
    const res = await fetch(`${http_url}/auth-check`, {
      credentials: "include",
    });
    const data = await res.json();

    if (data.status === "ok") {
      username = data.username;
      ConnectSocket();

      const usernameElement = document.getElementById("username");
      usernameElement.textContent = username;
    } else {
      window.location.href = "./auth.html";
    }
  } catch (err) {
    console.error("Auth check failed:", err);
    window.location.href = "./auth.html";
  }
}



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
let lastPingTime;
let latency
function ConnectSocket() {
    ToggleStatus(0);
    socket = new WebSocket(`${ws_url}/chat`);
    socket.onopen = () => {
        socket.send(JSON.stringify({ type: "fetch_dm_list" }));
        ToggleStatus(1);
        //restoreDMs();

        pingInterval = setInterval(() => {
            if (socket.readyState === WebSocket.OPEN) {
                lastPingTime = Date.now();
                socket.send("ping");
            }
        }, 600000);
        
    }
    socket.onclose = () => {
        ToggleStatus(-1);
        clearInterval(pingInterval); 
    };

    socket.onmessage = (event) => {
        const parsed = JSON.parse(event.data);

        //not working
        if (parsed.type === "pingcheck") {
            latency = Date.now() - lastPingTime;
            if(parsed.manual === true){
                displayMessage(JSON.stringify(["SERVER", `Your ping is ${latency}ms`, "server", null]));
                return;
            }
            console.log(latency)
            return;
        }
        
        
    
        if (parsed.type === "dm_list") {
            for (const username of parsed.users) {
                createDM(username); 
            }
            return;
        }

        //send msg history here
        if (parsed.type === "load_prev_msgs") {
                TakeALoadOfThis(parsed.msgs); 
                return
            }
            
            //individual messages
            displayMessage(event.data);

            if (document.hidden) {
                notificationSound.play().catch(console.warn);
            }
            if(parsed.type===NaN) {
            console.warn("Unknown message format:", parsed);
        }
    };
    
}

let current_view = "server";
let dm_recipient = null;

window.sendMessage =sendMessage
function sendMessage(event) {
    event.preventDefault();
    const MessageBox = document.getElementById("ClientMessage");
    const msg = MessageBox.value.trim();
    if (msg) {
        const payload = {
            type: current_view,
            msg: msg
        };
        if (current_view === "dm") {
            payload.to = dm_recipient;
        }
        if(msg==="/ping"){
    lastPingTime = Date.now();

        }
        socket.send(JSON.stringify(payload));
        MessageBox.value = "";
    }
}



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

function switchToDM(username) {
    const chatTitle = document.getElementById('ChatTitle');
    const messageInput = document.getElementById('ClientMessage');
    const messagesDiv = document.getElementById("ChatBox");

    if (dm_recipient !== username) {
        current_view = username ? "dm" : "server";
        dm_recipient = username;

        // Start fade out
        messagesDiv.classList.add("fade-transition");
        chatBox.innerHTML = '';
        showLoadingSkeleton();
        
        setTimeout(() => {
            LastMsgSender=""
            if (username) {
                socket.send(JSON.stringify({ type: "load_dm", with: username }));
            } else {
                socket.send(JSON.stringify({ type: "load_server"}));
            }

            chatTitle.textContent = username ? `${dm_recipient}` : 'Server Chat';
            messageInput.placeholder = username ? `Message ${dm_recipient}...` : `Type a message...`;

            // Fade in
            messagesDiv.classList.remove("fade-transition");
        }, 200); // Wait for fade-out to finish
    }

    requestAnimationFrame(() => {
        //messagesDiv.scrollTop = messagesDiv.scrollHeight;
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


// === Server button resets to public chat ===
document.querySelector('.server-btn:first-child').addEventListener('click', () => {
    switchToDM(null);
});
//LOADING WHILE SWITCHING DMS
function showLoadingSkeleton() {
    const chatBox = document.getElementById('ChatBox');
    //chatBox.innerHTML = '';
    const loadingMsg = document.createElement("p");
    loadingMsg.textContent = "Loading messages";
    loadingMsg.classList.add("LoadingMessage"); 
    chatBox.prepend(loadingMsg);
    let dotCount = 0;
    const interval = setInterval(() => {
        dotCount = (dotCount + 1) % 4;
        loadingMsg.textContent = "Loading messages" + ".".repeat(dotCount);
    }, 150);
    //remove in 3 secs
    setTimeout(() => {
        clearInterval(interval);
        loadingMsg.remove();
    }, 1000);
}

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



//CREATING FORMATTED MESSAGES
function createMesssageElement(sender, message , timestamp) {
    const messageContainer = document.createElement("div");
    messageContainer.classList.add("fade-in");
    messageContainer.dataset.timestamp = timestamp; 
    if (sender !== LastMsgSender) {
        const messageUser = document.createElement("button");
        messageUser.textContent = sender;
        messageContainer.appendChild(messageUser);
        LastMsgSender = sender;
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


let LastMsgSender=""
//FUNCTION CALLED WHEN NEW MESSAGE ARRIVES
function displayMessage(text) {
    let data;
    try {
        data = JSON.parse(text);
    } catch {
        return;
    }
    
    console.log("Parsed data:", data, "Type:", typeof data);
    let [user, message, type, to, timestamp, loadHistory] = data;
    const isDM = type === "dm";

    if (isDM) {
        if (dm_recipient !== user && dm_recipient !== to) {
            console.log(`Skipping DM message not in current view: ${user} -> ${to}`);
            createDM(user);
            notificationSound.play().catch(console.warn);
            return;
        }
    } else if (current_view === "dm") {

        console.log(`Skipping public message while in DM view`);
        notificationSound.play().catch(console.warn);
        return;
    }

    const messagesDiv = document.getElementById("ChatBox");

    if (!timestamp) timestamp = new Date().toISOString();

        const element = createMesssageElement(user, message, timestamp);
        messagesDiv.appendChild(element);
        requestAnimationFrame(() => {
            element.classList.add("show"); 
        });
        messagesDiv.scrollTop = messagesDiv.scrollHeight;
    //}
    
}

function TakeALoadOfThis(msgs) {
    const messagesDiv = document.getElementById("ChatBox");
    const wrapper = document.createElement("div");
    wrapper.classList.add("ChatBoxClone"); // use the same styles as ChatBox
    const elements=[];

    let timestamp;
    for (const msg of msgs) {
        const [user, message, type, withUser, msgTimestamp] = msg;
        const element = createMesssageElement(user, message, msgTimestamp);
        wrapper.appendChild(element);
        elements.push(element); // collect them for animation
        timestamp = msgTimestamp;
    }

    //store scroll position and disable scroll 
    const topVisible = messagesDiv.firstElementChild;
    const topOffset = topVisible?.getBoundingClientRect().top;
    messagesDiv.style.overflow = "hidden";

    messagesDiv.prepend(wrapper); 

    //restore scroll position and enable scroll again
    requestAnimationFrame(() => {
        if (topVisible) {
            const newOffset = topVisible.getBoundingClientRect().top;
            messagesDiv.scrollTop += newOffset - topOffset;
            for (const el of elements) {
                el.classList.add("show"); // fade them in
            }

        }
    });

    messagesDiv.style.overflow = "auto";
    if (timestamp) {
        Oldesttimestamp = timestamp;
    }
}



let Oldesttimestamp;
const chatBox = document.getElementById("ChatBox");
chatBox.addEventListener("scroll", () => {
    if (chatBox.scrollTop === 0 ) {
        let oldestElement = chatBox.firstElementChild;
       
        while (oldestElement && !oldestElement.dataset.timestamp) {
            oldestElement = oldestElement.firstElementChild;
        }
        if (!oldestElement) return;
        Oldesttimestamp = oldestElement.dataset.timestamp;
        if (!Oldesttimestamp) return;

        if (current_view === "dm") {
            showLoadingSkeleton()
            socket.send(JSON.stringify({
                type: "load_dm",
                with: dm_recipient,
                timestamp: Oldesttimestamp
            }));
        } 
        else if (current_view === "server") {
            showLoadingSkeleton()

            socket.send(JSON.stringify({
                type: "load_server",
                timestamp: Oldesttimestamp
            }));

        }
    }
})

//EMOJIS
window.insertRandomEmoji = insertRandomEmoji
async function insertRandomEmoji() {
    const res = await fetch("https://unpkg.com/emoji.json/emoji.json");
    const emoji = (await res.json())[Math.floor(Math.random() * 6500)].char;
    const input = document.getElementById("ClientMessage");
    input.setRangeText(emoji, input.selectionStart, input.selectionEnd, "end");
    //input.focus();
    //document.getElementById("emoji-button").textContent = emoji;
  }


const logoutButton = document.getElementById("logout-button");
if (logoutButton) {
  logoutButton.addEventListener("click", () => {
    document.cookie = "session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;";
    window.location.href = "/auth.html"; // Redirect to login page
  })}


  //PHONE

  function setVh() {
    let vh = window.innerHeight * 0.01;
    document.documentElement.style.setProperty('--vh', `${vh}px`);
  }
  
  window.addEventListener('resize', setVh);
  window.addEventListener('orientationchange', setVh);
  
  const sidebar = document.getElementById('sidebar-container');
  const mainContent = document.getElementById('main-content');
  let touchStartX = 0;
  let touchEndX = 0;

  // Swipe detection
  function handleTouchStart(e) {
    touchStartX = e.changedTouches[0].screenX;
  }

  function handleTouchEnd(e) {
    touchEndX = e.changedTouches[0].screenX;
    const deltaX = touchEndX - touchStartX;

    if (deltaX > 50) {
      // swipe right
      sidebar.classList.add('active');
    } else if (deltaX < -50) {
      // swipe left
      sidebar.classList.remove('active');
    }
  }

  // Tap outside sidebar to close
  document.addEventListener('click', (e) => {
    sidebar.classList.remove('active');
  });

  document.addEventListener('touchstart', handleTouchStart, false);
  document.addEventListener('touchend', handleTouchEnd, false);
