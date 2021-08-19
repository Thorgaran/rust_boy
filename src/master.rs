use crate::{controls, dma, hardware, instructions, interrupts, timer};
use std::io::{stdin, stdout, Read, Write};

const H_BLANK: u8 = 0;
const V_BLANK: u8 = 1;
const PX_TRANSFER: u8 = 2;

pub struct Master {
    pub nb_steps: u64,
    pub tick: u64,
    pub mode: u8,
    pub previous_mode: u8,
    pub step_by_step: bool,
    pub line_by_line: bool,
    pub screen_by_screen: bool,
    pub log: bool,
}

impl Master {
    pub fn step(
        &mut self,
        cpu: &mut hardware::Cpu,
        timer: &mut timer::Timer,
        controls: &mut controls::Controls,
        ram: &mut [u8; 0x10000],
    ) {
        self.nb_steps += 1;

        interrupts::interrupt_check(cpu, ram);
        cpu.clear_ticks();

        let instruct = instructions::Instruct::fetch(cpu, ram[cpu.pc as usize], ram[cpu.pc.wrapping_add(1) as usize]);
        cpu.pc = cpu.pc.wrapping_add(1);

        // println!("Step: {:#08}, PC: {:#06x}, OPCODE:{:#04x} => {:#04x} | {:#04x} | {:#04x} ({})", self.nb_steps, cpu.pc, instruct.opcode, 
        //     ram[(cpu.pc + 0) as usize], ram[(cpu.pc + 1) as usize], ram[(cpu.pc + 2) as usize], instruct.inst,
        // );

        if self.step_by_step {
            self.log = true;
            self.maxi_debug_print(&cpu, &timer, &ram, &controls, &instruct);
            wait();
        }
        
        self.tick = self.tick.wrapping_add(instruct.ticks as u64);
        self.tick = self.tick.wrapping_add((cpu.get_ticks()) as u64);

        timer.update(instruct.ticks, ram);

        controls.update_ram(ram);
        
        cpu.update_interrupt_status(); // If instruction from last step wants to change MIE

        instruct.inst.exec(cpu, ram);

        //adding temporary ticks from the cpu

        dma::update_dma(ram);
    }

    pub fn screen(
        &mut self,
        cpu: &mut hardware::Cpu,
        gpu: &mut hardware::Gpu,
        timer: &mut timer::Timer,
        controls: &mut controls::Controls,
        ram: &mut [u8; 0x10000],
    ) {
        ram[0xFF44] = 0;
        for i in 0..144 {
            while self.tick < 114 {
                if self.tick > 63 {
                    self.mode = H_BLANK;
                } else {
                    self.mode = PX_TRANSFER;
                }
                //print!("{esc}c", esc = 27 as char);
                //println!("SCREEN STATE__________________________________");
                //println!("State: Printing");
                //println!("Line: {}",i);
                //println!("Mode: {}",self.mode);
                //println!(" ");
                self.step(cpu, timer, controls, ram);
                self.lcd_stat(i, ram);
                if self.step_by_step {
                    wait();
                }
            }
            self.tick = 0;
            gpu.push_line(ram);

            if self.line_by_line {
                wait();
            }

            ram[0xff44] += 1;
        }

        ram[0xFF0F] = ram[0xFF0F] | 0b1;
        self.mode = V_BLANK;

        for _j in 0..10 {
            while self.tick < 114 {
                //print!("{esc}c", esc = 27 as char);
                //println!("SCREEN STATE__________________________________");
                //println!("State: V-Blank");
                //println!("Mode: {}",self.mode);
                //println!(" ");
                self.step(cpu, timer, controls, ram);
                self.lcd_stat(254, ram);
                if self.step_by_step {
                    wait();
                }
            }
            self.tick = 0;
            if self.line_by_line {
                wait();
            }
            ram[0xff44] += 1;
        }

        if self.screen_by_screen {
            wait();
        }
    }

    pub fn maxi_debug_print(
        &self,
        cpu: &hardware::Cpu,
        timer: &timer::Timer,
        ram: &[u8; 0x10000],
        controls: &controls::Controls,
        instruct: &instructions::Instruct,
    ) {
        if self.log {
            println!("Pc: {:#06x}", cpu.pc);
            println!("OPERATION____________________________________");
            println!("NB steps:{}", self.nb_steps);
            println!("Count:{}", self.tick);
            println!("Pc: {:#06x}", cpu.pc);
            println!(
                "Ram values: {:#04x} {:#04x} {:#04x}",
                ram[cpu.pc as usize],
                ram[(cpu.pc + 1) as usize],
                ram[(cpu.pc + 2) as usize]
            );
            println!("Opcode:{:#04x}", &instruct.opcode);
            println!("Name:{}", &instruct.inst);
            println!("Instruction: {}", &instruct.desc);
            println!("Ticks: {}", &instruct.ticks);
            println!();
            println!("a:{} / {:#04x}", cpu.a, cpu.a);
            println!("f:{} / {:#04x}", cpu.f, cpu.f);
            println!("b:{} / {:#04x}", cpu.b, cpu.b);
            println!("c:{} / {:#04x}", cpu.c, cpu.c);
            println!("d:{} / {:#04x}", cpu.d, cpu.d);
            println!("e:{} / {:#04x}", cpu.e, cpu.e);
            println!("h:{} / {:#04x}", cpu.h, cpu.h);
            println!("l:{} / {:#04x}", cpu.l, cpu.l);
            println!("sp:{:#04x}", cpu.sp);
            println!(
                "Stack values: {:#04x} {:#04x} {:#04x}",
                ram[cpu.sp as usize],
                ram[(cpu.sp + 1) as usize],
                ram[(cpu.sp + 2) as usize]
            );
            println!("mie: {}", cpu.mie);
            println!("0xFFFF: {:#010b}", ram[0xFFFF]);
            println!("0xFF0F: {:#010b}", ram[0xFF0F]);
            println!();
            println!("FLAGS STATE__________________________________");
            println!("Z:{}", cpu.get_flag(hardware::Flag::Z));
            println!("N:{}", cpu.get_flag(hardware::Flag::N));
            println!("H:{}", cpu.get_flag(hardware::Flag::H));
            println!("C:{}", cpu.get_flag(hardware::Flag::C));
            println!();
            println!("TIMER STATE__________________________________");
            println!("Divider:{:#04x}", ram[0xff04]);
            println!("Divider ticks:{}", timer.divider_ticks);
            println!("Timer enable:{}", timer.timer_enb);
            println!("Timer division:{}", timer.division);
            println!("Timer:{:#04x}", ram[0xff05]);
            println!("Timer ticks:{}", timer.timer_ticks);
            println!();
            println!("INPUT STATE__________________________________");
            println!(
                "Buttons: U:{} D:{} L:{} R:{} A:{} B:{} SE:{} ST:{}",
                controls.up,
                controls.down,
                controls.left,
                controls.right,
                controls.a,
                controls.b,
                controls.select,
                controls.start
            );
            println!("0XFF00: {:#010b}", ram[0xFF00]);
            println!();
            println!("WARNING______________________________________");
        }
    }

    pub fn lcd_stat(&mut self, line: u8, ram: &mut [u8; 0x10000]) {
        if ram[0xFF41] & 0b0100000 > 0 && line == ram[0xFF45] && self.previous_mode == H_BLANK {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: LY=LYC");}
            self.previous_mode = self.mode;
        }
        if ram[0xFF41] & 0b00001000 > 0 && self.mode == H_BLANK && self.mode != self.previous_mode {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: H_BLANK");}
            self.previous_mode = self.mode;
        }
        if ram[0xFF41] & 0b00010000 > 0 && self.mode == V_BLANK && self.mode != self.previous_mode {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: V_BLANK");}
            self.previous_mode = self.mode;
        }
        if ram[0xFF41] & 0b0010000 > 0
            && self.mode == PX_TRANSFER
            && self.mode != self.previous_mode
        {
            ram[0xFF0F] = ram[0xFF0F] | 0b00000010;
            //if self.log {println!("/!\\ STAT interrupt trigerred: PX_TRANSFER");}
            self.previous_mode = self.mode;
        }
    }
}

pub fn wait() {
    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
    print!("{esc}c", esc = 27 as char);
}
