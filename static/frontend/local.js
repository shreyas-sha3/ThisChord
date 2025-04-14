
// //FOR LOCAL PUBLIC CHAT STORAGE
// function saveChat(username, messages) {
//     const trimmed = messages.slice(-30); 
//     localStorage.setItem(`chat_${username || 'public'}`, JSON.stringify(trimmed));
// }
// //FOR LOCAL PUBLIC CHAT STORAGE
// function loadChat(username) {
//     return JSON.parse(localStorage.getItem(`chat_${username || 'public'}`)) || [];
// }

// //ONLY FOR LOCAL PUBLIC CHAT DISPLAY CBA TO CHANGE
// function renderChatMessages(messages) {
//     const chatBox = document.getElementById("ChatBox");
//     chatBox.innerHTML = ""; // Clear existing messages
//     lastUser = "";

//     messages.forEach(msg => {
//         if (typeof msg === "string") {
//             const fallback = document.createElement("div");
//             fallback.textContent = msg;
//             chatBox.appendChild(fallback);
//             return;
//         }
//         const { sender, message,timestamp  } = msg;
//         console.log(msg)
//         const element = createMessageElement(sender, message,timestamp);
//         chatBox.appendChild(element);
//     });

//     chatBox.scrollTop = chatBox.scrollHeight;
// }

// function restoreDMs() {
//     const dmList = JSON.parse(localStorage.getItem("dmList") || "[]");
//     dmList.forEach(username => createDM(username));
// }