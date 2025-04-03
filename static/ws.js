let socket
const statusElement = document.getElementById("connection-status");

ConnectSocket()

function ToggleStatus(status){
    if(status==1){
    statusElement.textContent = "Connected!";
    statusElement.style.color="#b8bb26";
    }
    else if(status==0){
        statusElement.textContent = "Connecting...";
        statusElement.style.color="#83a598";
    }
    else if(status==-1){
        statusElement.textContent = "Disconnected...(Refresh)";
        statusElement.style.color="#fb4934";
    }
}

function ConnectSocket() {
    ToggleStatus(0);
    socket = new WebSocket("wss://rust-chat-um86.onrender.com/chat");
    socket.onopen = () => {
        ToggleStatus(1);
    };
}

socket.onclose = () => {
    ToggleStatus(-1);
};

socket.onmessage = (event) =>{ console.log("Message received:", event.data);
displayMessage(event.data);
}


function sendMessage(event){
    event.preventDefault();
    let MessageBox = document.getElementById("ClientMessage");
    socket.send(MessageBox.value);
    displayMessage(MessageBox.value);
    MessageBox.value = "";
}
lastUser=""
function displayMessage(text) {
        let user,message,messageContainer;

        try {
            [user, message] = JSON.parse(text);
        } catch {
            message = text;
        }
        
        let messagesDiv = document.getElementById("ChatBox");
        messageContainer = document.createElement("div");

        if(user!=lastUser){
            messageUser = document.createElement("h4");
            messageUser.textContent = user || "You";
            messageContainer.appendChild(messageUser);
            lastUser=user;
        }

        let messageContent = document.createElement("p");
        messageContent.textContent = message;        
        messageContainer.appendChild(messageContent);
        messagesDiv.appendChild(messageContainer);

        messageContainer.classList.add(
            user === "SERVER" ? "ServerMessage" : (user ? "OtherMessage" : "SelfMessage")
        );        
        messagesDiv.scrollTop = messagesDiv.scrollHeight;

    }
