# Web server

Implementation of a Web server / client using Rust. 

## :pushpin: Functionalities
 - Http query parser ( Server ) 
 - Screen sharing ( Server / Client )




# :postbox: Http query parser
A simple HTTP query parser allowing the server to retrieve : 
* The Http request **start line**
* The Http request **headers**
* The Http request optional **body**


# :vhs: Screen Sharing

**Screen sharing functionality sent through UDP without RTP overhead.**

---

## 🔧 UDP Video Streaming Workflow - End to End

> **System Overview:** Real-time video streaming system using UDP protocol with subscription-based client management and NAL unit processing.

---

## 🔄 Complete Workflow

### 1. 📡 **Client Subscription Management**

**Connection Process:**  
Clients connect to the UDP server socket *(hardcoded for now)*

#### **Subscription Protocol:**

**📥 Subscribe Operation:**
```
Message Format: 0x01
Data Size: 1 byte
Action: Add client to streaming list
```

**📤 Unsubscribe Operation:**
```
Message Format: 0x02 0x00
Data Size: 2 bytes  
Action: Remove client from streaming list
```

#### **Current Limitations:**
> ⚠️ **Server Configuration**: Server address is hardcoded

---

### 2. 💾 **Server Client Cache Management**
   
   Server maintains active client registry:
   - 🗂️ **Client Storage**: IP addresses + port mappings

---

### 3. 📺 **Video Frame Broadcasting Pipeline**
   
   Server continuously processes and distributes video frames:
   
   #### 3.1 🎬 **Frame Acquisition**
   - Capture raw video frames from source
   - Apply timestamp metadata
   
   #### 3.2 🔐 **Frame Encoding**
   - Convert frames to H.264/H.265 format
   - Apply compression settings
   - Generate encoding metadata
   
   #### 3.3 ✂️ **NAL Unit Segmentation**
   - Parse encoded data into NAL units
   - **NAL Types processed:**
     - `Type 1`: Non-IDR slice
     - `Type 5`: IDR slice (keyframe)
     - `Type 7`: SPS (Sequence Parameter Set)
     - `Type 8`: PPS (Picture Parameter Set)
   
   #### 3.4 📡 **UDP Transmission**
   - Fragment large NAL units for UDP
   - Timestamp packet generation for order
   - Broadcast to all subscribed clients

---

### 4. 📥 **Client Reception & Processing Pipeline**
   
   Multi-stage client-side processing:
   
   #### 4.1 📦 **Raw Data Reception**
   - 🔄 **Buffer Management**: Store unordered UDP packets

   #### 4.2 🔄 **Data Reorganization**  
   - 📏 **Threshold Trigger**: Process when buffer reaches optimal size
   - 🗂️ **Sequence Sorting**: Reorder packets by sequence number
   - 🧩 **Fragment Assembly**: Reconstruct complete NAL units
   
   #### 4.3 🔓 **Video Decoding**
   - 🎯 **NAL Processing**: Parse reconstructed NAL units
   - 🎨 **Frame Reconstruction**: Decode H.264/H.265 to raw frames
   - 🖼️ **Color Space Conversion**: Convert to display format
   
   #### 4.4 🖥️ **Frame Display**
   - 🎬 **Rendering**: Display decoded frames

---

## 📊 Video Streaming Flow Diagram

```
                    SERVER SIDE
                    ───────────
    [Video Source] 
          │
          ▼
    ┌─────────────────────┐
    │  Global Buffer      │ ◄── Store frames
    │  [F1][F2][F3][F4]   │
    └─────────┬───────────┘
              │ Dequeue one element
              ▼
    ┌─────────────────────┐
    │   UDP Transmitter   │ ◄── Send to all clients
    │    Broadcasting     │
    └─────────┬───────────┘
              │
              ▼
       UDP Network Layer
    ═══════════════════════
              │
              ▼
                    CLIENT SIDE
                    ──────────
    ┌─────────────────────┐
    │  First Buffer       │ ◄── Receive unordered
    │  [F3][F1][F4][F2]   │     packets
    └─────────┬───────────┘
              │ When threshold reached
              ▼
    ┌─────────────────────┐
    │   Sorting Process   │ ◄── Reorder packets
    │      [F1→F2→F3→F4]  │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │  Sorted Buffer      │ ◄── Ordered frames
    │  [F1][F2][F3][F4]   │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │  Frame Decoder      │ ◄── Decode & display
    │    & Display        │
    └─────────────────────┘
              │
              ▼
         [Screen Output]
```


---

## 🛠️ **Technical Stack**

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Transport** | UDP Sockets | Low-latency data transmission |
| **Video Codec** | H.264/H.265 | Efficient video compression |
| **Packetization** | NAL Units | Network-friendly video segments |
| **Buffering** | Multi-stage Buffers | Packet reordering & loss recovery |


