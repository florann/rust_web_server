use std::{net::UdpSocket, sync::{mpsc::{Receiver, Sender}, Arc}, thread::{self, JoinHandle}, time::Duration};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};
use pixels::{wgpu, Pixels, SurfaceTexture};
use egui_wgpu::ScreenDescriptor;

use crate::{models::structs::gpu_decoder::GpuDecoder, GLOBAL_SORTED, SERVER_ADDRESS};
use crate::GLOBAL_BUFFER;
use crate::BUFFER_LEN_BEFORE_PROCESS;
use crate::MAX_UDP_PACKET_SIZE;

pub struct App <'a>{
    pub pixels: Option<Pixels<'a>>,
    pub decoder: GpuDecoder,
    pub window: Option<Arc<Window>>,
    pub socket: Arc<UdpSocket>,
    pub sort_sender: Sender<()>,
    pub sort_receiver: Receiver<()>,
    pub handler_receiver_thread: Option<JoinHandle<()>>,
    pub handler_sorter_thread: Option<JoinHandle<()>>,
    // egui
    pub egui_ctx: Option<egui::Context>,
    pub egui_state: Option<egui_winit::State>,
    pub egui_renderer: Option<egui_wgpu::Renderer>,
}

impl<'a> App<'a> {
   pub fn run(mut self) {
        let event_loop = EventLoop::new().unwrap();
        
        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);
            
            match event {
                Event::Resumed => {
                    if self.window.is_none() {
                        self.initialize_window(elwt);
                    }
                }
                Event::WindowEvent { event, .. } => {
                    // Let egui handle it first
                    if let Some(egui_state) = &mut self.egui_state {
                        if let Some(window) = &self.window {
                         let response = egui_state.on_window_event(&**window, &event);
                            if response.consumed {
                                return;
                            }
                        }
                    }
                    
                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::RedrawRequested => {
                            self.update();
                            self.draw();
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        }).unwrap();
    }

    fn initialize_window(&mut self, elwt: &EventLoopWindowTarget<()>) {
        let window = WindowBuilder::new()
            .with_title("Video Stream")
            .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080))
            .build(elwt)
            .unwrap();
        
        let arc_window = Arc::new(window);
        
        // Create SurfaceTexture from window
        let surface_texture = SurfaceTexture::new(1920, 1080, arc_window.clone());
        
        // Pass SurfaceTexture to Pixels::new()
        let pixels = Pixels::new(1920, 1080, surface_texture).unwrap();
        
        // Initialize egui
        let egui_ctx = egui::Context::default();
        let mut egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &arc_window,
            None,
            None
        );

        // Create egui renderer
        let egui_renderer = egui_wgpu::Renderer::new(
            pixels.device(),
            pixels.render_texture_format(),
            None,
            1,
        );

        self.pixels = Some(pixels);
        self.window = Some(arc_window);
        self.egui_ctx = Some(egui_ctx);
        self.egui_state = Some(egui_state);
        self.egui_renderer = Some(egui_renderer);
    }

    fn update(&mut self) {
        // Process NAL units from the sorted queue
        while let Some((_timestamp, nal_data)) = GLOBAL_SORTED.lock().unwrap().pop_front() {
            let nal_type = nal_data[4] & 0x1F;

            if nal_type == 9 {
                break;
            }

            println!("Sending to decoder {}", nal_type);

            // Feed NAL to decoder
            match self.decoder.decode_udp_packet(nal_data) {
                Ok(is_success) => {
                    println!("xxxxxxx Success decoding xxxxxx");
                    if is_success {
                        if let Some(pixels) = &mut self.pixels {
                            match self.decoder.get_rgba_data() {
                                Ok(rgba) => {
                                    let frame_buffer = pixels.frame_mut();
                                    let copy_len = frame_buffer.len().min(rgba.len());
                                    frame_buffer[..copy_len].copy_from_slice(&rgba[..copy_len]);
                                    thread::sleep(Duration::from_millis(33));
                                },
                                Err(_err) => {}
                            }
                        }
                        break; // Process one frame per update
                    }
                },
                Err(_err) => {}
            }
        }
    }

    fn draw(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(pixels) = &mut self.pixels else { return };
        let Some(egui_ctx) = &self.egui_ctx else { return };
        
        // Prepare egui frame
        let Some(egui_state) = &mut self.egui_state else { return };
        let raw_input = egui_state.take_egui_input(window.as_ref());

        let Some(egui_ctx) = &self.egui_ctx else { return };
        let full_output = egui_ctx.run(raw_input, |ctx| {
            // Top menu bar
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Connect to Server").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            std::process::exit(0);
                        }
                    });
                    ui.menu_button("Help", |ui| {
                        if ui.button("About").clicked() {
                            ui.close_menu();
                        }
                    });
                });
            });
        });

        // Handle platform output (e.g., cursor changes)
        egui_state.handle_platform_output(window.as_ref(), full_output.platform_output);

        let Some(egui_renderer) = &mut self.egui_renderer else { return };
        // Render pixels first, then egui on top
        let render_result = pixels.render_with(|encoder, render_target, context| {
            // Render the pixel buffer
            context.scaling_renderer.render(encoder, render_target);

            // Prepare egui render
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [1920, 1080],
                pixels_per_point: window.scale_factor() as f32,
            };

            // Render egui on top
            let paint_jobs = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            
            for (id, image_delta) in &full_output.textures_delta.set {
                egui_renderer.update_texture(
                    &context.device,
                    &context.queue,
                    *id,
                    image_delta,
                );
            }

            egui_renderer.update_buffers(
                &context.device,
                &context.queue,
                encoder,
                &paint_jobs,
                &screen_descriptor,
            );

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
            }

            // Cleanup textures
            for id in &full_output.textures_delta.free {
                egui_renderer.free_texture(id);
            }

            Ok(())
        });

        if let Err(err) = render_result {
            eprintln!("Render error: {}", err);
        }
    }

    // Function to subscribe to server
    pub fn subscribe_to_server(&self) {
        let opt = SERVER_ADDRESS.lock().unwrap().clone();
        if let Some(addr) = opt {
            let subscribe_message = [1u8];
            self.socket.send_to(&subscribe_message, addr).unwrap();
        }
    }

    // Spawn thread to receive data
    pub fn spawn_receiver_thread(&mut self, sort_sender: Sender<()>) {
        let copy_socket = self.socket.clone();

        let handler_receiver_thread = thread::spawn(move || {
            let mut udp_buffer = vec![0u8; MAX_UDP_PACKET_SIZE];
            let mut chunk_buffer = Vec::new();

            loop {
                match copy_socket.recv(&mut udp_buffer) {
                    Ok(nb_bytes) => {
                        match Self::receive_packet(&sort_sender, &udp_buffer, &mut chunk_buffer, nb_bytes) {
                            Ok(()) => (),
                            Err(err) => {
                                println!("Error: Receive packet {}", err);
                            }
                        }
                    },
                    Err(error) => {
                        eprintln!("Socket recv error: {}", error);
                    }
                }
            }
        });
        self.handler_receiver_thread = Some(handler_receiver_thread);
    }

    // Spawn thread to sort data
    pub fn spawn_sorter_thread(&mut self, sort_receiver: Receiver<()>) {
        let handler_sorter_thread = thread::spawn(move || {
            loop {
                match sort_receiver.recv() {
                    Ok(()) => {
                        let mut buffer = GLOBAL_BUFFER.lock().unwrap();
                        if buffer.len() > BUFFER_LEN_BEFORE_PROCESS {
                            let mut global_buffer_drain: Vec<(u128, Vec<u8>)> = buffer.drain(0..31).collect();
                            drop(buffer);

                            global_buffer_drain.sort_by_key(|key| key.0);
                            GLOBAL_SORTED.lock().unwrap().extend(global_buffer_drain);
                        }
                    },
                    Err(err) => {
                        println!("Try receive error {}", err);
                    }
                }
            }
        });
        self.handler_sorter_thread = Some(handler_sorter_thread);
    }

    // Receive packets
    fn receive_packet(sender: &Sender<()>, udp_buffer: &Vec<u8>, chunk_buffer: &mut Vec<u8>, nb_bytes: usize) -> Result<(), String> {
        // If chunked
        if udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0x0F])
            || udp_buffer.starts_with(&[0x01, 0x01, 0x01, 0xFF]) {
            let data: Vec<u8> = udp_buffer[4..nb_bytes].to_vec();
            if udp_buffer[3] == 0xFF {
                chunk_buffer.extend_from_slice(&data);
                let tuple = Self::parse_received_packet(chunk_buffer, chunk_buffer.len());
                *chunk_buffer = Vec::new();
                match Self::add_packet_to_receiver(sender, tuple) {
                    Ok(()) => {
                        return Ok(())
                    },
                    Err(error) => {
                        return Err(error);
                    }
                }
            } else {
                chunk_buffer.extend_from_slice(&data);
                println!("Size chunk - {}", chunk_buffer.len());
            }
            Ok(())
        } else {
            let tuple = Self::parse_received_packet(udp_buffer, nb_bytes);
            match Self::add_packet_to_receiver(sender, tuple) {
                Ok(()) => {
                    Ok(())
                },
                Err(error) => {
                    println!("Error: add_packet_to_receiver");
                    Err(error)
                }
            }
        }
    }

    // Parse received packets
    fn parse_received_packet(udp_buffer: &Vec<u8>, nb_bytes: usize) -> (u128, Vec<u8>) {
        let timestamp = u128::from_be_bytes(
            udp_buffer[0..16].try_into().unwrap()
        );
        
        let nal_data = udp_buffer[16..nb_bytes].to_vec();
        (timestamp, nal_data)
    }

    // Add packet to receiver buffer
    fn add_packet_to_receiver(sender: &Sender<()>, tuple: (u128, Vec<u8>)) -> Result<(), String> {
        match GLOBAL_BUFFER.lock() {
            Ok(mut global_buffer) => {
                global_buffer.push(tuple);
            },
            Err(err) => {
                return Err(err.to_string());
            }
        }

        match GLOBAL_BUFFER.lock() {
            Ok(global_buffer) => {
                if global_buffer.len() > BUFFER_LEN_BEFORE_PROCESS {
                    sender.send(()).ok();
                }
            },
            Err(err) => {
                return Err(err.to_string());
            }
        }
        Ok(())
    }
}

