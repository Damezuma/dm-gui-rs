extern crate winapi;
extern crate user32;
extern crate kernel32;
use winapi::windef::{HWND,RECT};
use winapi::minwindef::HMODULE;
use user32::GetWindowRect;

use winapi::LRESULT;
use winapi::LPARAM;
use winapi::WPARAM;
use winapi::UINT;
use winapi::LPCWSTR;
use winapi::HBRUSH;
use winapi::HINSTANCE;
use winapi::HICON;
use winapi::HCURSOR;
use winapi::HMENU;
use winapi::SW_SHOWNORMAL;
use winapi::DWORD;
use winapi::CW_USEDEFAULT;
use winapi::WS_EX_CLIENTEDGE;
use std::mem;


use winapi::winuser::WS_OVERLAPPEDWINDOW;
use winapi::winuser::WS_VISIBLE;
use winapi::winuser::WNDCLASSW;

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
trait Child:Component{
    
}
trait Container{
    fn add<T:Child +Sized + 'static>(&mut self,name:&str, component:T);
    fn find_child<T:Child>(&self, name:&str)->Option<&T>;
}
trait Sizer{
    fn layout<T:Child +Sized>(&self, children:&Vec<T>);
}
trait EventHandlable{
    fn on_message(&self, msg:UINT,wparam:WPARAM, lparam:LPARAM)->Option<LRESULT>;
    fn add(&self, handler:WindowEventHandler);
}
enum WindowEventHandler{
    OnPaint{
        handler:Box<Fn(&Window)>
    }
}
struct Window
{
    control:CommonWindowControl,
    children:Dict<String, Box<Child>>,
    handler:std::sync::RwLock<Vec<WindowEventHandler>>
}
impl EventHandlable for Window{
    fn add(&self, handler:WindowEventHandler){
        let mut writer = self.handler.write().unwrap();
        writer.push(handler);
    }
    fn on_message(&self, msg:UINT,wparam:WPARAM, lparam:LPARAM)->Option<LRESULT>{
        return None;
    }
}
impl Container for Window{
    fn add<T:Child +Sized + 'static>(&mut self, name:&str, component:T){
        //TODO:
        self.children.insert(String::from(name),Box::new(component));
    }
    fn find_child<T:Child>(&self, name:&str)->Option<&T>{
        if let Some(v) = self.children.get(name){
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
struct WindowBuilder{
    hinstance:HINSTANCE,
    topmost:Option<bool>,
    position:Option<Point>,
    size:Option<Size>,
    title:Option<String>
}
impl WindowBuilder{
    fn new(system:&System)->WindowBuilder{
        WindowBuilder{
            hinstance:system.hinstance,
            topmost:None,
            position:None,
            size:None,
            title:None
        }
    }
    fn is_topmost(mut self, value:bool)->WindowBuilder{
        self.topmost = Some(value);
        return self;
    }
    fn position(mut self, value:Point)->WindowBuilder{
        self.position = Some(value);
        return self;
    }
    fn size(mut self, value:Size)->WindowBuilder{
        self.size = Some(value);
        return self;
    }
    fn title(mut self, value:&str)->WindowBuilder{
        self.title = Some(String::from(value));
        return self;
    }
    fn build(self)->Result<Window, ()>{
        let class_name = to_wide("dm-gui-rm-window");
        let title = if let Some(v) = self.title{
            to_wstring(v.as_str())
        }
        else{
            to_wstring("")
        };
        let (cx, cy) = if let Some(v) = self.size{
            (v.get_width(), v.get_height())
        }
        else{
            (CW_USEDEFAULT, CW_USEDEFAULT)
        };
        let (x,y) = if let Some(v) = self.position{
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
            return Err(());
        }
        else{
            let win = Window{
                children:Dict::new(),
                handler:std::sync::RwLock::new(Vec::new()),
                control:CommonWindowControl{
                    hwnd:h_wnd_window
                }
            };
            unsafe{
                let hi = std::mem::transmute::<&EventHandlable,[winapi::HANDLE; 2]>(&win);
                user32::SetPropW(h_wnd_window,TEXT("a").as_ptr(), hi[0]);
                user32::SetPropW(h_wnd_window,TEXT("b").as_ptr(), hi[1]);
            }
            return Ok(win);
        }
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
    fn create<Parent:Container + WindowComponent>(parent:&mut Parent, name:&'static str, text:&str)->bool{
        let parent_hwnd = parent.get_hwnd();
        
        println!("{}",parent_hwnd as u64);
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
    fn set_title(&self, title:&str){
        unsafe{

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
    if let Some(v) =handler.on_message(msg, w_param,l_param){
        return v;
    }
    else{
        return user32::DefWindowProcW(h_wnd, msg, w_param, l_param);
    }
    match msg{
        winapi::winuser::WM_DESTROY=>{
            user32::PostQuitMessage(0);
        },
        winapi::winuser::WM_COMMAND=>{
            if l_param != 0{
                let hwnd = l_param as HWND;
            }
        },
        _=>{
            
        }
    }
    return 0;
}

#[derive(Clone)]
struct System{
    hinstance:HMODULE
}
impl System{
    fn init()->Result<System, ()>{
        
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
                cbWndExtra: mem::size_of::<&EventHandlable>() as i32,
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
            hinstance:hmodule
        };
        return Ok(res);
    }
    fn message_loop(){
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
            }
        }
    }
}

fn main() {
      // Here our unsafe code goes - 
    println!("{}",mem::size_of::<&Component>());
    let system = System::init().unwrap();
    let mut window = WindowBuilder::new(&system).title("Rust Window").build().unwrap();
    window.show();
    Button::create(&mut window,"id_button_1", "button1");
    Button::create(&mut window,"id_button_2", "button2");
    let btn:&Button = window.find_child::<Button>("id_button_1").unwrap();
    let btn2:&Button = window.find_child::<Button>("id_button_2").unwrap();
    btn.show();
    btn2.show();
    btn.set_size(Size::new(400,600));
    btn2.set_position(Point::new(0,0));
    btn2.set_position(Point::new(400,0));
    System::message_loop();
}
