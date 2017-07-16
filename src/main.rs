extern crate winapi;
extern crate user32;
extern crate kernel32;
use winapi::windef::{HWND,RECT};
use winapi::minwindef::HMODULE;
use user32::GetWindowRect;

use winapi::{LRESULT,LPARAM,WPARAM,UINT,LPCWSTR,HBRUSH,HINSTANCE,HICON,HCURSOR,HMENU,SW_SHOWNORMAL,DWORD,CW_USEDEFAULT,WS_EX_CLIENTEDGE};
use winapi::winuser::{WS_OVERLAPPEDWINDOW,WS_VISIBLE,WNDCLASSW};

use std::mem;
use std::sync::{Arc, RwLock, Mutex,Once, ONCE_INIT};
use std::ffi::OsStr;
use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::collections::HashMap as Dict;
use std::borrow::BorrowMut;
fn to_wstring(str : &str) -> Vec<u16> {
    let v : Vec<u16> =
            OsStr::new(str).encode_wide().chain(Some(0).into_iter()).collect();
    v
}
fn to_wide(msg:&str) -> Vec<u16> {
   OsStr::new(msg).encode_wide().chain(once(0)).collect()
}
fn TEXT(msg:&str) -> Vec<u16> {
    OsStr::new(msg).encode_wide().chain(once(0)).collect()
}
fn hide_console_window() 
{
    let window = unsafe {
        kernel32::GetConsoleWindow()
    };

    if window != std::ptr::null_mut() {
        unsafe {
            user32::ShowWindow (window, winapi::SW_HIDE)
        };
    }
}

#[derive(Clone)]
struct Point{
    x:i32,
    y:i32
}
impl Point{
    fn new(x:i32, y:i32)->Point{
        Point{x:x,y:y}
    }
    fn get_x(&self)->i32{
        return self.x;
    }
    fn get_y(&self)->i32{
        return self.y;
    }
}
#[derive(Clone)]
struct Size{
    width:i32,
    height:i32
}
impl Size{
    fn new(w:i32, h:i32)->Size{
        Size{width:w,height:h}
    }
    fn get_width(&self)->i32{
        return self.width;
    }
    fn get_height(&self)->i32{
        return self.height;
    }
}
trait Component{
    fn show(&self);
    fn get_size(&self)->Size;
    fn set_size(&self,size:Size);
    fn set_position(&self, pos:Point);
}

trait WindowComponent{
    fn get_hwnd(&self)->HWND;
}
trait Child:Component + WindowComponent{
    
}
trait Container{
    fn add<T:Child +Sized + 'static>(&self,name:&str, component:T);
    fn find_child<T:Child>(&self, name:&str)->Option<&T>;
}
trait Sizer{
    fn layout<T:Child +Sized>(&self, children:&Vec<T>);
}
trait EventHandlable{
    fn on_message(&self, msg:UINT,wparam:WPARAM, lparam:LPARAM)->Option<LRESULT>;
    fn connect(&self, handler:WindowEvent);
}
struct PaintEvent{

}
struct MouseEvent{
    mouse_pos:Point
}
struct Event{

}
impl MouseEvent{
    fn new(mouse_pos:Point)->MouseEvent{
        MouseEvent{mouse_pos:mouse_pos}
    }
    fn get_position(&self)->Point{
        self.mouse_pos.clone()
    }
}
enum WindowEvent{
    OnPaint(Box<Fn(&Window, &mut PaintEvent)>),
    OnMouseMove(Box<Fn(&Window, &mut MouseEvent)>),
    OnButtonClick(String, Box<Fn(&Window, &mut Event)>)
}
impl WindowEvent{
    fn paint<T:'static+ Fn(&Window, &mut PaintEvent)>( it:T)->WindowEvent{
        WindowEvent::OnPaint(Box::new(it))
    }
    fn mousemove<T:'static+ Fn(&Window, &mut MouseEvent)>( it:T)->WindowEvent{
        WindowEvent::OnMouseMove(Box::new(it))
    }
    fn button_click<T:'static+ Fn(&Window, &mut Event)>(name:&str, f:T)->WindowEvent{
        WindowEvent::OnButtonClick(String::from(name), Box::new(f))
    }
}
struct Window
{
    control:CommonWindowControl,
    children:RwLock<Dict<String, Box<Child>>>,
    handler:RwLock<Vec<WindowEvent>>
}
impl Window{
    fn create<Initializer:Fn(&Window)>(system:&System, opt:WindowOpt, initializer:Initializer)->bool{
        let class_name = to_wide("dm-gui-rm-window");
        let title = if let Some(v) = opt.title{
            to_wstring(v.as_str())
        }
        else{
            to_wstring("")
        };
        let (cx, cy) = if let Some(v) = opt.size{
            (v.get_width(), v.get_height())
        }
        else{
            (CW_USEDEFAULT, CW_USEDEFAULT)
        };
        let (x,y) = if let Some(v) = opt.position{
            (v.get_x(), v.get_y())
        }
        else{
            (CW_USEDEFAULT, CW_USEDEFAULT)
        };
        let h_wnd_window = unsafe{
            user32::CreateWindowExW(0, class_name.as_ptr(), 
                title.as_ptr(), WS_OVERLAPPEDWINDOW, 
                x, y, cx, cy, 0 as HWND, 0 as HMENU, 0 as HINSTANCE, std::ptr::null_mut())
        };
        if h_wnd_window as u64== 0{
            return false;
        }
        else{
            let win =Box::new(Window{
                children:RwLock::new(Dict::new()),
                handler:RwLock::new(Vec::new()),
                control:CommonWindowControl{
                    hwnd:h_wnd_window
                }
            });
            unsafe{
                let hi = std::mem::transmute::<&EventHandlable,[winapi::HANDLE; 2]>(win.as_ref());
                user32::SetPropW(h_wnd_window,TEXT("a").as_ptr(), hi[0]);
                user32::SetPropW(h_wnd_window,TEXT("b").as_ptr(), hi[1]);
            }
            initializer(&win);
            let mut v = system.windows.write().unwrap();
            v.push(win);
            
            return true;
        }
    }
    fn show_message(&self, title:&str, text:&str){
        unsafe{
            user32::MessageBoxW(self.get_hwnd(), TEXT(text).as_ptr(), TEXT(title).as_ptr(), 0);
        }
    }
}
impl EventHandlable for Window{
    fn connect(&self, handler:WindowEvent){
        let mut writer = self.handler.write().unwrap();
        writer.push(handler);
    }
    fn on_message(&self, msg:UINT,wparam:WPARAM, lparam:LPARAM)->Option<LRESULT>{
        let reader =match self.handler.read(){
            Ok(v)=>v,
            Err(e)=>{
                println!("{:?}",e);
                return None;
            }
        };
        match msg{
            winapi::winuser::WM_COMMAND=>{
                let children = self.children.read().unwrap();
                println!("LINE:let children = self.children.read().unwrap();");
                for (key, it) in children.iter(){
                    let h =unsafe{std::mem::transmute::<&Box<Child>, &Box<Button>>(it)};
                    println!("h is {:x} hwnd is {:x}",h.get_hwnd() as LPARAM, lparam);
                    if lparam as HWND == h.get_hwnd(){
                        println!("LINE:if lparam as HWND == h.get_hwnd(){{");
                        for it in reader.iter(){
                           match *it{
                                WindowEvent::OnButtonClick(ref name, ref handler)=>if key == name{
                                    handler(self, &mut Event{});
                                    return Some(0);
                                },
                                _=>{}
                            }
                       }
                       break;
                    }
                }
            },
            winapi::winuser::WM_PAINT=>{
                for it in reader.iter(){
                    match *it{
                        WindowEvent::OnPaint(ref handler)=>{
                            handler(self, &mut PaintEvent{});
                            return Some(0);
                        },
                        _=>{}
                    }
                };
            }
            winapi::winuser::WM_MOUSEMOVE=>{
                let x = winapi::windowsx::GET_X_LPARAM(lparam);
                let y = winapi::windowsx::GET_Y_LPARAM(lparam);
                
                let mut event = MouseEvent::new(Point::new(x,y));
                for it in reader.iter(){
                    match *it{
                        WindowEvent::OnMouseMove(ref handler)=>{
                            handler(self, &mut event);
                            return Some(0);
                        },
                        _=>{}
                    }
                };
            }
            _=>{}
        }
        return None;
    }
}
impl Container for Window{
    fn add<T:Child +Sized + 'static>(&self, name:&str, component:T){
        //TODO:
        let mut chwrite = self.children.write().unwrap();
        chwrite.insert(String::from(name),Box::new(component));
    }
    fn find_child<T:Child>(&self, name:&str)->Option<&T>{
        let chreader = self.children.read().unwrap();
        if let Some(v) = chreader.get(name){
            let rs =unsafe{std::mem::transmute::<&Box<Child>, &Box<T>>(v)};
            return Some(rs.as_ref());
        }
        return None;
    }
}

impl WindowComponent for Window{
    fn get_hwnd(&self)->HWND{
        return self.control.get_hwnd();
    }
}
impl Component for Window{
    
    fn show(&self){
        self.control.show();
    }
    fn get_size(&self)->Size{
        return self.control.get_size();
    }
    fn set_size(&self,size:Size){
        return self.control.set_size(size);
    }
    fn set_position(&self, pos:Point){
        return self.control.set_position(pos);
    }
}
struct WindowOpt{
    topmost:Option<bool>,
    position:Option<Point>,
    size:Option<Size>,
    title:Option<String>
}
impl WindowOpt{
    fn new()->WindowOpt{
        WindowOpt{
            topmost:None,
            position:None,
            size:None,
            title:None
        }
    }
    fn is_topmost(mut self, value:bool)->WindowOpt{
        self.topmost = Some(value);
        return self;
    }
    fn position(mut self, value:Point)->WindowOpt{
        self.position = Some(value);
        return self;
    }
    fn size(mut self, value:Size)->WindowOpt{
        self.size = Some(value);
        return self;
    }
    fn title(mut self, value:&str)->WindowOpt{
        self.title = Some(String::from(value));
        return self;
    }
}
struct CommonWindowControl{
    hwnd:HWND
}
impl WindowComponent for CommonWindowControl{
    fn get_hwnd(&self)->HWND{
        return self.hwnd;
    }
}
impl Component for CommonWindowControl{
    
	fn show(&self){
		unsafe{
			user32::ShowWindow(self.hwnd, winapi::SW_SHOW);  
		}
	}
    fn get_size(&self)->Size{
        let mut rc = RECT{left: 0, top: 0, right: 0, bottom: 0};
        unsafe{
            GetWindowRect(self.hwnd, &mut rc);
        }
        return Size{
            height:rc.bottom - rc.top,
            width:rc.right - rc.left
        };
    }
    fn set_size(&self,size:Size){
        unsafe{
            user32::SetWindowPos(self.hwnd, 0 as HWND, 0,0, size.get_width(), size.get_height(), winapi::winuser::SWP_NOMOVE);
        }
    }
    fn set_position(&self, pos:Point){
        unsafe{
            user32::SetWindowPos(self.hwnd, 0 as HWND, pos.get_x(), pos.get_y() , 0, 0, winapi::winuser::SWP_NOSIZE);
        }
    }
}
enum ButtonEvent{
    OnClicked{handler:Box<Fn(&Window)>}
}
struct Button{
    control:CommonWindowControl
}

impl Button{
    fn create<Parent:Container + WindowComponent>(parent:&Parent, name:&'static str, text:&str)->bool{
        let parent_hwnd = parent.get_hwnd();
        
        let hwnd = unsafe{
            user32::CreateWindowExW(0,TEXT("button").as_ptr(),TEXT(text).as_ptr(),winapi::winuser::WS_CHILD | winapi::winuser::BS_PUSHBUTTON, 0,0,50,100,parent_hwnd,0 as HMENU, 0 as HINSTANCE, std::ptr::null_mut())
        };
        if hwnd as u64 == 0{
            return false;
        }
        else{
            let button= Button{
                control:CommonWindowControl{
                    hwnd:hwnd
                }
            };
            parent.add(name, button);
            return true;
        }
    }
}
impl Button{
    fn set_text(&self, title:&str){
        unsafe{
            user32::SetWindowTextW(self.control.get_hwnd(), TEXT(title).as_ptr());
        }
    }
}
impl Child for Button{

}
impl WindowComponent for Button{
    fn get_hwnd(&self)->HWND{
        return self.control.get_hwnd();
    }
}

impl Component for Button{
    fn show(&self){
        self.control.show();
    }
    fn get_size(&self)->Size{
        return self.control.get_size();
    }
    fn set_size(&self,size:Size){
        return self.control.set_size(size);
    }
    fn set_position(&self, pos:Point){
        return self.control.set_position(pos);
    }
    
}
struct TextEdit{
    control:CommonWindowControl
}
impl TextEdit{
    fn create<Parent:Container + WindowComponent>(parent:&Parent, name:&'static str)->bool{
        let parent_hwnd = parent.get_hwnd();
        let hwnd = unsafe{
            user32::CreateWindowExW(0,TEXT("edit").as_ptr(),std::ptr::null_mut(),winapi::winuser::WS_CHILD | winapi::winuser::WS_BORDER, 0,0,50,100,parent_hwnd,0 as HMENU, 0 as HINSTANCE, std::ptr::null_mut())
        };
        if hwnd as u64 == 0{
            return false;
        }
        else{
            let button= Button{
                control:CommonWindowControl{
                    hwnd:hwnd
                }
            };
            parent.add(name, button);
            return true;
        }
    }
}
impl Child for TextEdit{

}
impl WindowComponent for TextEdit{
    fn get_hwnd(&self)->HWND{
        return self.control.get_hwnd();
    }
}
impl Component for TextEdit{
    fn show(&self){
        self.control.show();
    }
    fn get_size(&self)->Size{
        return self.control.get_size();
    }
    fn set_size(&self,size:Size){
        return self.control.set_size(size);
    }
    fn set_position(&self, pos:Point){
        return self.control.set_position(pos);
    }
    
}
pub unsafe extern "system" fn win_proc(h_wnd :HWND, msg :UINT, w_param :WPARAM, l_param :LPARAM ) -> LRESULT {
    let handler:&EventHandlable = unsafe{
        let mut t:[winapi::HANDLE;2]=[
            user32::GetPropW(h_wnd, TEXT("a").as_ptr()),
            user32::GetPropW(h_wnd, TEXT("b").as_ptr())
        ];
        if t[0] as u64 == 0  && t[0] as u64 == 0{
            return user32::DefWindowProcW(h_wnd, msg, w_param, l_param);
        }
        std::mem::transmute::<[winapi::HANDLE;2],&EventHandlable>(t)
    };
    if winapi::winuser::WM_DESTROY == msg{
        let system = singleton().inner;
        let mut write = system.windows.write().unwrap();
        //println!("{}",write.len());
        {
            let hwnd = h_wnd;
            let l = write.len();
            for i in 0..l{
                let b = write.get(i).unwrap().get_hwnd();
                if b == hwnd{
                    write.remove(i);
                    return 0;
                }
            }
        }
    }
    if let Some(v) =handler.on_message(msg, w_param,l_param){
        return v;
    }
    else{
        return user32::DefWindowProcW(h_wnd, msg, w_param, l_param);
    }
}
#[derive(Clone)]
struct SystemReader {
    // Since we will be used in many threads, we need to protect
    // concurrent access
    inner: Arc<System>
}

fn singleton() -> SystemReader {
    // Initialize it to a null value
    static mut SINGLETON: *const SystemReader = 0 as *const SystemReader;
    static ONCE: Once = ONCE_INIT;

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = SystemReader {
                inner: Arc::new(System{
                    hinstance:0 as HMODULE,
                    windows:Arc::new(RwLock::new(Vec::new()))
                })
            };

            // Put it in the heap so it can outlive this call
            SINGLETON = mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
}
#[derive(Clone)]
struct System{
    hinstance:HMODULE,
    windows:Arc<RwLock<Vec<Box<Window>>>>
}

impl System{
    fn init()->Result<SystemReader,()>{
        
        let hmodule:HMODULE = unsafe{
            kernel32::GetModuleHandleW(0 as *const u16)
        };
        if hmodule as u32 == 0{
            return Err( () );
        }
        let class_name = to_wide("dm-gui-rm-window");
        
        let ret = unsafe {
            let wnd = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(win_proc), 
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as HINSTANCE,
                hIcon: user32::LoadIconW(0 as HINSTANCE, winapi::winuser::IDI_APPLICATION),
                hCursor: user32::LoadCursorW(0 as HINSTANCE, winapi::winuser::IDI_APPLICATION),
                hbrBackground: 16 as HBRUSH,
                lpszMenuName: 0 as LPCWSTR,
                lpszClassName: class_name.as_ptr(),
            };
            user32::RegisterClassW(&wnd)    
        };
        if ret == 0 {
            return Err(()  );
        }
        //hide_console_window();
        let res = System{
            windows:Arc::new(RwLock::new(Vec::new())),
            hinstance:hmodule
        };
        return Ok(singleton());
    }
    fn message_loop(&self){
        unsafe 
        {
            let mut msg = winapi::winuser::MSG {
                hwnd : 0 as HWND,
                message : 0 as UINT,
                wParam : 0 as WPARAM,
                lParam : 0 as LPARAM,
                time : 0 as DWORD,
                pt : winapi::windef::POINT { x: 0, y: 0, },
            };
            loop
            {   
                let pm = user32::GetMessageW(&mut msg, 0 as HWND, 0, 0);
                if pm == 0 {
                    break;
                }
                
                if msg.message == winapi::winuser::WM_QUIT {
                    break;
                }
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
                //println!("{:X}",msg.message);
                let is_quick = {self.windows.read().unwrap().len() == 0};
                if is_quick{
                    user32::PostQuitMessage(0);
                }
            }
        }
    }
}

fn main() {
      // Here our unsafe code goes - 
    let a = System::init().unwrap();
    let winopt = WindowOpt::new().title("Rust Window");
    Window::create(&a.inner, winopt,|window|{
        window.show();
        Button::create(window,"id_button_1", "button1");
        Button::create(window,"id_button_2", "button2");
        TextEdit::create(window,"id_text");
        let btn:&Button = window.find_child::<Button>("id_button_1").unwrap();
        let text_edit:&TextEdit = window.find_child::<TextEdit>("id_text").unwrap();
        println!("btn is {:x}",btn.get_hwnd() as LPARAM);
        //let btn2:&Button = window.find_child::<Button>("id_button_2").unwrap();
        btn.show();
        //btn2.show();
        btn.set_size(Size::new(400,200));
        btn.set_position(Point::new(0,0));
        //btn2.set_position(Point::new(400,0));
        //btn2.set_size(Size::new(400,200));
        btn.set_text(&format!("XY"));
        text_edit.show();
        text_edit.set_size(Size::new(400,25));
        text_edit.set_position(Point::new(400,0));
        window.connect(WindowEvent::mousemove(|window, event|{
            let pos = event.get_position();
            let btn:&Button = window.find_child::<Button>("id_button_1").unwrap();
            btn.set_text(&format!("X:{}, Y:{}",pos.get_x(), pos.get_y()));
        }));
        window.connect(WindowEvent::button_click("id_button_1",|window, event|{
            window.show_message("알림","버튼1이 눌렸습니다.");
        }));
    });
    a.inner.message_loop();
}
