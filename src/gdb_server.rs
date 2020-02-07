use std::str;
use std::io::Write;
use std::io::Read;
use std::net::TcpListener;

    ///PACKET_SIZE - Размер GDB-RSP-пакета в байтах ("PacketSize=PACKET_SIZE" в ответ на qSupported)
    ///Размер должен вмещать все GPR регистры + символ 'G'
    const PACKET_SIZE: usize = 2048; //Уточнить, возможно имеет смысл сделать побольше !!!!!!!!
    ///BUF_SIZE - Размер буфера под TCP-пакет от GDB (В 2 раза больше просто на всякий случай) //???????
    const BUF_SIZE: usize = PACKET_SIZE * 2;

pub struct RspPacket<'a>
{
    pub len: Option<usize>,                         // Длина принятого RSP-пакета
    pub src_packet: Option<&'a str>,                // Исходный RSP-пакет в utf-8: (+/-)$<data>#cs
    pub data: Option<&'a str>,                      // Только данные <data> из RSP-пакета (между первым '$' и последним '#')
    pub first_cmd_symbol: Option<char>,             // Первый символ данных data[0]
    pub last_ack_sign: Option<char>,                // Acknowledgment '+' или '-' для предыдущего пакета (если есть). На случай, если no-acknowledgment режим еще не включен
    pub only_symb: Option<bool>,                    // Признак того, что это не пакет, а одиночный acknowledgment '+'/'-' или управляющий символ (Ctrl+C)
    pub cs: Option<&'a str>,                        // Контрольная сумма RSP-пакета
    pub need_responce: Option<bool>,                // Признак необходимости ответа. Без need_responce не обойтись т.к. в конструкторе заранее неизвостно, что будет содержать responce
    pub responce: Option<String>,                   // Ответный RSP-пакет
    pub output_text: Option<String>,                // Текстовое сообщение для вывода в GDB-консоль. Допустимо только с Stop Reply Packet и qRcmd !!
    pub kill_flag: Option<bool>,                    // Признак команды 'vKill'
}


impl<'a> RspPacket<'a>
{
    ///Конструктор
    pub fn new(input_buf: &'a[u8], input_len: usize) -> RspPacket<'a>
    {
        match input_len
        {
            2...PACKET_SIZE => //Диапазоны в образцах включительные
            { //if input_len > 1 : Пакет $data#cs, а не одиночный символ
                let usd_pos = str::from_utf8(&input_buf[0..input_len]).unwrap() .find('$').unwrap();
                let sharp_pos = str::from_utf8(&input_buf[0..input_len]).unwrap() .find('#').unwrap(); //Или .rfind() для быстроты
                println!("usd_pos: {}", usd_pos);//Убрать!
                println!("sharp_pos: {}", sharp_pos);//Убрать!

                RspPacket{
                    len: Some(input_len),
                    src_packet: str::from_utf8(&input_buf[0 .. input_len]).ok(),
                    data: str::from_utf8(&input_buf[usd_pos+1 .. sharp_pos]).ok(),
                    first_cmd_symbol: Some( char::from(input_buf[usd_pos+1]) ),
                    last_ack_sign: if let 1 = usd_pos {Some(char::from(input_buf[0]))} else{None},
                    only_symb: Some(false),
                    cs: str::from_utf8(&input_buf[sharp_pos+1 .. sharp_pos+3]).ok(),
                    need_responce: Some(true), //Признак может быть сброшен в зависимости от пришедшей команды (только в случае, если это Пакет)
                    responce: None, //Ответ будет сформирован при необходимости
                    output_text: None,
                    kill_flag: Some(false),
                }
            }
            1 =>
            { //if 1 == input_len : Не пакет, а одиночный acknowledgment '+'/'-' или управляющий символ (Ctrl+C)
                RspPacket{
                    len: Some(input_len),
                    src_packet: str::from_utf8(&input_buf[0..input_len]).ok(),
                    data: None,
                    first_cmd_symbol: None,
                    last_ack_sign: Some(char::from(input_buf[0])),
                    only_symb: Some(true),
                    cs: None,
                    need_responce: Some(true), //На '+' надо ответить '+'; На '-' надо повторить последний пакет; На Ctrl+C - Stop Reply Packet
                    responce: None, //Ответ будет сформирован при необходимости
                    output_text: None,
                    kill_flag: Some(false),
                }
            }
            0 => 
            { //Пустое сообщение (input_len = 0)
                RspPacket{
                    len: Some(0),
                    src_packet: None,
                    data: None,
                    first_cmd_symbol: None,
                    last_ack_sign: None,
                    only_symb: None,
                    cs: None,
                    need_responce: Some(false), //Игнорировать пустое сообщение
                    responce: None,
                    output_text: None,
                    kill_flag: Some(false),
                }
            }
            _ =>
            { //Такого не должно быть, так как input_len типа usize
                panic!("Исключение в конструкторе структуры RspPacket: Некорректное значение input_len");
            }
        }//match
    }


    ///Сформировать ответный RSP-пакет: обернуть содержимое сообщения-ответа в $ и #cs
    ///Использовать только если необходимо. Ненужно например для одиночного Ack '+' или '-'
    fn responce_add_usd_cs(& mut self, msg_str: &str)
    {
        //Создание строки с выделением буфера. Это не должно ничего замедлить т.к. для одной команды ответ формируется максимум один раз.
        self.responce = Some(String::with_capacity(PACKET_SIZE)); //Ответный RSP-пакет не должен быть длиннее PACKET_SIZE

        let mut checksum: u8 = 0;
        for c in msg_str.as_bytes()
        {
            checksum = checksum.wrapping_add(*c);
        }
        self.responce = Some(format!("${}#{:02x}", msg_str, checksum)); //cs: {:02x} - шестнадцатиричное u8 как строка и дополнить лидирующим нулем до двух цифр
        //Тут нельзя передать в responce срез &str на String. Т.к. время жизни String, возвращаемой из format!() ограничено вызывающей функцией

        let l = match self.responce //.unwrap() не работает: error[E0507]: cannot move out of borrowed content
                {
                    Some(ref v) => v, //а так работает потому что тут ссылка (ref) на String
                    None => panic!("RspPacket.responce = None"),
                };

        if l.len() > PACKET_SIZE //Длина уже с учетом $ и #cs. Длина строки в байтах
        {
            panic!("Формирование ответного RSP-пакета: пакет длиннее чем PACKET_SIZE. len = {}. PACKET_SIZE = {}", l.len() , PACKET_SIZE);
        }
    }


    ///Сформировать Otext RSP-пакет: Сформировать строку из ASCII-кодов исходного сообщения и обернуть её в $O и #cs
    ///$O<console_output_text>#cs
    fn text_add_usd_o_cs(& mut self, msg_str: &str)
    {
        //Создание строки с выделением буфера
        self.output_text = Some(String::with_capacity(PACKET_SIZE)); //console_output_text RSP-пакет не должен быть длиннее PACKET_SIZE
        let mut otext = String::with_capacity(PACKET_SIZE);

        //Цикл для формирования строки otext, состоящих из ASCII-кодов символов строкового среза msg_str
        for c in msg_str.as_bytes()
        {
            otext = otext + &format!("{:02x}", *c); //otext содержит значения *u8 в Hex
        }

        otext.insert(0, 'O'); //Добавить 'O' в начало otext
        //Строка otext будет длинее, чем исходный срез msg_str. Так как на каждый символ среза msg_str будет приходиться по два символа (двузначное число в Hex) в строке otext
        //И еще надо учесть 'O' в начале otext. Поэтому для подсчета cs для otext нужен отдельный цикл
        let mut checksum: u8 = 0;
        for c in otext.as_bytes()
        {
            checksum = checksum.wrapping_add(*c);
        }

        self.output_text = Some(format!("${}#{:02x}", otext, checksum));

        let l = match self.output_text
                {
                    Some(ref v) => v,
                    None => panic!("RspPacket.output_text = None"),
                };

        if l.len() > PACKET_SIZE //Длина уже с учетом $O и #cs. Длина строки в байтах
        {
            panic!("Формирование output_text RSP-пакета: пакет длиннее чем PACKET_SIZE. len = {}. PACKET_SIZE = {}", l.len() , PACKET_SIZE);
        }
    }


    ///Сформировать monitor-команду (текст) из ASCII-кодов, содержащихся в &str
    //Возвращается String потому что внутри функции модифицируется строка
    //Исходный срез cmd_str содержит последовательность двухзначных ASCII-коды (в Hex) символов. А в результате должна получиться строка String этих символов
    //Если измерять в u8, то результирующая String в два раза короче исходного cmd_str. Потому что на каждый итоговый символ приходится две Hex-цифры исходного ASCII-кода
    //Алгоритм работы:
            //Из исходного строкового среза cmd_str формируется вектор подсрезов (по два u8 каждый) - двухзначные ASCII-коды;
            //Итератор по подсрезам: 
                //Получить строковый срез из подсреза(два u8);
                //Получить само значение ASCII-кода, сохранить в u8;
                //u8 привести к char и присоединить в конец String.
    fn extract_monitor_cmd(cmd_str: &str) -> String
    {
        let mut result_cmd = String::with_capacity(PACKET_SIZE); //Создание строки с выделением буфера
        let mut one_symb_ascii_str; //Строковый срез str. Двухзначный ASCII-код одного символа из начального среза cmd_str
        let mut one_symb_ascii_u8: u8 = 0;

        let str_by_2_u8 = cmd_str.as_bytes().chunks(2); //Из среза cmd_str.as_bytes() сформировать вектор непересекающихся подсрезов по два u8

        for subslice in str_by_2_u8 //Итератор по подсрезам (по два u8)
        {
            one_symb_ascii_str = str::from_utf8(&subslice).unwrap(); //Получение строкового среза из подсреза u8 (из двух u8)
            one_symb_ascii_u8 = u8::from_str_radix(one_symb_ascii_str, 16).unwrap(); //Получить само значение ASCII-кода из его исходного представления в HEX виде
            result_cmd.push(char::from(one_symb_ascii_u8)); //Получить char из u8. И присоединить к результирующей строке String
        }
        result_cmd
    }


    ///Сформировать ответ без $ и #cs например для одиночного Ack '+' или '-'
    ///Можно использовать для $OK#9a и для $#00 т.к. responce() быстрее, чем responce_add_usd_cs()
    fn responce(& mut self, msg_str: &str)
    {
        self.responce = Some(String::with_capacity(PACKET_SIZE)); //Ответный RSP-пакет не должен быть длиннее PACKET_SIZE

        self.responce = Some(msg_str.to_string());

        let l = match self.responce
                {
                    Some(ref v) => v,
                    None => panic!("RspPacket.responce = None"),
                };

        if l.len() > PACKET_SIZE //Вряд ли вообще будет такой длинный ответ без $ и #cs
        {
            panic!("Формирование ответного RSP-пакета: пакет длиннее чем PACKET_SIZE. len = {}. PACKET_SIZE = {}", l.len(), PACKET_SIZE);
        }
    }


    ///Обработка полученной команды
    fn match_cmd(&mut self)
    {
        match self.first_cmd_symbol.unwrap()
        {
            '?'=>
            {
                //Запрос состояние цели (причины останова)
                //$?
                println!("GDB-Server : Получена команда ?");
                //Если цель остановлена (halt), ответить S05 = SIGTRAP
                //Что делать если цель не остановлена? $''#00 или S00 или не отвечать ????
                //Возможно добавить S02 = SIGINT
                //...
                self.responce("$T05#b9");
                self.need_responce = Some(true);
            },

            'g'=>
            {
                //Чтение всех регистров общего назначения
                //$g
                println!("GDB-Server : Получена команда g");
                //...
                self.responce_add_usd_cs("00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff");
                self.need_responce = Some(true);
            },

            'G'=>
            {
                //Запись всех регистров общего назначения
                //$G<байты>
                println!("GDB-Server : Получена команда G");
                //...
                //Искать .find("G") только в начале (для скорости)
                self.responce("$OK#9a");
                self.need_responce = Some(true);
            },

            'p'=>
            {
                //Чтение произвольного регистра
                //$p<n>
                println!("GDB-Server : Получена команда p. Номер регистра {}", usize::from_str_radix(&self.data.unwrap()[1..], 16).unwrap());
                self.responce_add_usd_cs("0a00011000000000");
                self.need_responce = Some(true);
            },

            'P'=>
            {
                //Запись произвольного регистра
                //$P<n>=<байты>
                let eq_pos = self.data.unwrap().find("=").unwrap(); //Позиция знака '=' для определения номера регистра
                let reg_num = usize::from_str_radix(&self.data.unwrap()[1..eq_pos], 16).unwrap();
                let reg_val = usize::from_str_radix(&self.data.unwrap()[eq_pos+1..], 16).unwrap(); //Значение перевернуто!
                println!("GDB-Server : Получена команда P. Номер регистра {}. Значение = {}", reg_num, reg_val);
                self.responce("$OK#9a");
                self.need_responce = Some(true);
            },

            'm'=>
            {
                //Чтение памяти
                //$m<addr>,<len>
                let comma_pos = self.data.unwrap().find(",").unwrap(); //Позиция знака ',' для определения адреса
                let addr = usize::from_str_radix(&self.data.unwrap()[1..comma_pos], 16).unwrap();
                let bytes_len = usize::from_str_radix(&self.data.unwrap()[comma_pos+1..], 16).unwrap();
                println!("GDB-Server : Получена команда m. Адрес = 0x{:x}. Количество байт для чтения = {}", addr, bytes_len);
                self.responce_add_usd_cs("00112233");
                self.need_responce = Some(true);
            },

            'X' | 'M'=>
            {
                //Запись в память
                //$M<addr>,<len>:<data>
                let comma_pos = self.data.unwrap().find(",").unwrap(); //Позиция знака ',' для определения адреса
                let addr = usize::from_str_radix(&self.data.unwrap()[1..comma_pos], 16).unwrap();
                let colon_pos = self.data.unwrap().find(":").unwrap(); //Позиция знака ':' для определения числа байт
                let bytes_len = usize::from_str_radix(&self.data.unwrap()[comma_pos+1..colon_pos], 16).unwrap();
                if bytes_len == 0
                {//Пробный пустой пакет "X0,0:"
                    println!("GDB-Server : Получена команда M. Адрес = 0x{:x}. Количество байт для записи = {}.", addr, bytes_len);
                }
                else
                {
                    let bytes = usize::from_str_radix(&self.data.unwrap()[colon_pos+1..], 16).unwrap();
                    println!("GDB-Server : Получена команда M. Адрес = 0x{:x}. Количество байт для записи = {}. Байты для записи = 0x{:x}.", addr, bytes_len, bytes);
                }
                self.responce("$OK#9a");
                self.need_responce = Some(true);
            },

            'z'=>
            {
                //Снятие matchpoint
                //$z<type>,<addr>,<kind>
                let addr_pos = 3; //Позиция addr = Позиция первой ',' +1
                let kind_pos = self.data.unwrap()[addr_pos..].find(",").unwrap() +1; //Позиция kind = Позиция второй ',' относительно addr_pos +1
                let addr = usize::from_str_radix(&self.data.unwrap()[addr_pos..addr_pos+kind_pos-1], 16).unwrap();
                let kind = usize::from_str_radix(&self.data.unwrap()[addr_pos+kind_pos..], 16).unwrap();
                println!("GDB-Server : Получена команда z. addr = 0x{:x}. kind = {}", addr, kind);

                match &self.data.unwrap()[1..2] //type
                {
                    "0"=>
                    {//software breakpoint
                        println!("GDB-Server : Получена команда z0");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "1"=>
                    {//hardware breakpoint
                        println!("GDB-Server : Получена команда z1");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "2"=>
                    {//write watchpoint
                        println!("GDB-Server : Получена команда z2");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "3"=>
                    {//read watchpoint
                        println!("GDB-Server : Получена команда z3");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "4"=>
                    {//access watch point
                        println!("GDB-Server : Получена команда z4");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    _=>
                    {
                        println!("GDB-Server : Unknown z-type: {}", &self.data.unwrap()[1..2]);
                        self.responce("+$#00");
                        self.need_responce = Some(true);
                    },
                }//match z-type
            },

            'Z'=>
            {
                //Установка matchpoint
                //$Z<type>,<addr>,<kind>
                let addr_pos = 3; //Позиция addr = Позиция первой ',' +1
                let kind_pos = self.data.unwrap()[addr_pos..].find(",").unwrap() +1; //Позиция kind = Позиция второй ',' относительно addr_pos +1
                let addr = usize::from_str_radix(&self.data.unwrap()[addr_pos..addr_pos+kind_pos-1], 16).unwrap();
                let kind = usize::from_str_radix(&self.data.unwrap()[addr_pos+kind_pos..], 16).unwrap(); //Если будут опциональные параметры (...[;cond_list...][;cmds:persist,cmd_list...]), то так работать не будет. kind надо будет выделять не до конца, а до первой ';'
                println!("GDB-Server : Получена команда Z. addr = 0x{:x}. kind = {}", addr, kind);

                match &self.data.unwrap()[1..2] //type
                {
                    "0"=>
                    {//software breakpoint
                        println!("GDB-Server : Получена команда Z0");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "1"=>
                    {//hardware breakpoint
                        println!("GDB-Server : Получена команда Z1");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "2"=>
                    {//write watchpoint
                        println!("GDB-Server : Получена команда Z2");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "3"=>
                    {//read watchpoint
                        println!("GDB-Server : Получена команда Z3");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    "4"=>
                    {//access watch point
                        println!("GDB-Server : Получена команда Z4");
                        //...
                        self.responce("$OK#9a");
                        self.need_responce = Some(true);
                    },
                    _=>
                    {
                        println!("GDB-Server : Unknown Z-type: {}", &self.data.unwrap()[1..2]);
                        self.responce("+$#00");
                        self.need_responce = Some(true);
                    },
                }//match Z-type
            },

            'q'=>
            {
                //Пакеты q-запросов не очень большие, так что можно искать contains() по всему пакету (не только в начале)
                if self.data.unwrap().contains("qSupported")
                {
                    println!("GDB-Server : Получена команда qSupported");
                    //'PacketSize=xx' обязательно.
                    //'QStartNoAckMode+' обязательно.
                    self.responce_add_usd_cs( &format!("PacketSize={:x};QStartNoAckMode+;vContSupported+", PACKET_SIZE) );
                    self.need_responce = Some(true);
                }
                else if self.data.unwrap().contains("qfThreadInfo")
                {
                    println!("GDB-Server : Получена команда qfThreadInfo");
                    //'l' - Конец списка потоков
                    self.responce_add_usd_cs("l");
                    self.need_responce = Some(true);
                }
                else if self.data.unwrap().contains("qC")
                {
                    println!("GDB-Server : Получена команда qC");
                    //Нулевой thread
                    self.responce_add_usd_cs("QC0");
                    self.need_responce = Some(true);
                }
                else if self.data.unwrap().contains("qAttached")
                {
                    println!("GDB-Server : Получена команда qAttached");
                    //Запрос: GDB-server подключается к существующему процессу или создает новый процесс?
                    //Команда связана с остановкой (и перезапуском) цели
                    self.responce_add_usd_cs("0");//0: по команде (gdb) quit GDB пришлет 'vKill'. 1: по команде (gdb) quit GDB пришлет 'D'(Detach).
                    self.need_responce = Some(true);
                }
                else if self.data.unwrap().contains("qSymbol")
                {
                    println!("GDB-Server : Получена команда qSymbol");
                    //Информация о символах не нужна
                    self.responce("$OK#9a");
                    self.need_responce = Some(true);
                }
                else if self.data.unwrap().contains("qOffsets")
                {
                    println!("GDB-Server : Получена команда qOffsets");
                    //Смещения секции при загрузке прошивки через GDB
                    self.responce_add_usd_cs("Text=0;Data=0;Bss=0");
                    self.need_responce = Some(true);
                }
                else if self.data.unwrap().contains("qRcmd")
                {
                    //Консольная команда 'monitor command'
                    //$qRcmd,command
                    //$Otext можно использовать только с Stop Reply Packet и с qRcmd !
                    //После $Otext обязательно должен быть $OK
                    let command = RspPacket::extract_monitor_cmd(&self.data.unwrap()[6..]); //Поле 'command' находится после ','
                    println!("GDB-Server : Получена команда qRcmd. command = \'{}\'", command);
                    match &command[..]
                    {
                        "reset init"=>
                        {
                            //...
                            self.text_add_usd_o_cs(" GDB-Server message : 'reset init' monitor command.\n + Any text message.\n");
                            println!("GDB-Server : 'reset init' monitor command");
                        },
                        "reset halt"=>
                        {
                            //...
                            self.text_add_usd_o_cs(" GDB-Server message : 'reset halt' monitor command.\n + Any text message.\n");
                            println!("GDB-Server : 'reset halt' monitor command");
                        },
                        _=>
                        {
                            self.text_add_usd_o_cs( &(" GDB-Server message : Unknown monitor command \'".to_string() + &command + "\'!\n") );
                            println!("GDB-Server : Unknown monitor command \'{}\'!", command);
                        },
                    }//match command
                    self.responce("$OK#9a");
                    self.need_responce = Some(true);
                }
                else
                {
                    println!("GDB-Server : Unknown command! (q-запрос)");
                    //Неподдерживаемая команда (q-запрос)
                    self.responce("+$#00");
                    self.need_responce = Some(true);
                }
            },

            'Q'=>
            {
                if self.data.unwrap().contains("QStartNoAckMode")
                {
                    println!("GDB-Server : Получена команда QStartNoAckMode");
                    //Дальше будем работать без подтверждений +/- (no-acknowledgment-режим)
                    self.responce("$OK#9a");
                    self.need_responce = Some(true);
                }
                else
                {
                    println!("GDB-Server : Unknown command! (Q-запрос)");
                    //Неподдерживаемая команда (Q-запрос)
                    self.responce("+$#00");
                    self.need_responce = Some(true);
                }
            },

            'v'=>
            {
                if self.data.unwrap().contains("vCont")
                {
                    match &self.data.unwrap()[0..6]
                    {
                        "vCont?"=> //if self.data.unwrap().contains("vCont?")
                        {//Запрос поддерживаемых vCont-action
                            println!("GDB-Server : Получена команда vCont?");
                            self.responce_add_usd_cs("vCont;c;C;s;S"); //GDB doesn't accept c without C and s without S
                            self.need_responce = Some(true);
                        }
                        "vCont;"=>
                        {//Команда к действию (vCont-action)
                            println!("GDB-Server : Получена команда vCont;");
                            //Наверно для работы в единственном потоке можно ориентироваться на первое vCont-action ';s' или ';c'
                            match &self.data.unwrap()[5..7]
                            {
                                ";c"=>
                                {//continue action
                                    println!("GDB-Server : vCont, c-action");
                                    //...
                                    self.responce("$T05#b9"); //Stop-reply packet
                                    //Плюс еще можно ответить Otext
                                    self.need_responce = Some(true);
                                },
                                ";s"=>
                                {//step action
                                    println!("GDB-Server : vCont, s-action");
                                    //...
                                    self.responce("$T05#b9"); //Stop-reply packet
                                    self.need_responce = Some(true);
                                },
                                _=>
                                {
                                    println!("GDB-Server : Unknown vCont action: {} !", &self.data.unwrap()[5..6]);
                                    self.responce("+$#00");
                                    self.need_responce = Some(true);
                                },
                            }//match vCont-action
                        },
                        _=>
                        {
                            println!("GDB-Server : Unknown vCont command!");
                            self.responce("+$#00");
                            self.need_responce = Some(true);
                        },
                    }//match vCont
                }
                else if self.data.unwrap().contains("vKill")
                {
                    println!("GDB-Server : Получена команда vKill");
                    self.responce("$OK#9a");
                    self.need_responce = Some(true);
                    self.kill_flag = Some(true);
                }
                else
                {
                    println!("GDB-Server : Unknown command! (v-запрос)");
                    //Неподдерживаемая команда (v-запрос)
                    //Здесь же обрабатывается имитация неподдерживаемой команды: $vMustReplyEmpty#3a
                    self.responce("+$#00");
                    self.need_responce = Some(true);
                }
            },

            _=>
            {
                println!("GDB-Server : Unknown command {}!", self.first_cmd_symbol.unwrap()); //Вывести "GDB-Server : Unknown command" в log
                //Неподдерживаемые команды. Ответ от GDB-сервера должен быть: $#00
                self.responce("+$#00");
                self.need_responce = Some(true);
            },
        }//match
    }
}//impl RspPacket


///GDB-Сервер
pub fn gdb_server()
{
    let addr = "127.0.0.1:9999";
    let listener = TcpListener::bind(addr).unwrap();
    println!("Server listening at {}", addr);
    let mut input_buf = [0x7Eu8; BUF_SIZE]; //Инициализация буфера символом '~'
    let mut input_len = 0; //usize

    for stream in listener.incoming() //stream типа TcpStream
    {
        let mut stream = stream.unwrap();
        loop
        {
            input_len = stream.read(&mut input_buf).unwrap();
            let mut rsp_pkt = RspPacket::new(&input_buf, input_len);

            if rsp_pkt.need_responce.unwrap()
            {//Ответ требуется
                if rsp_pkt.only_symb.unwrap()
                {//acknowledgment '+'/'-' или управляющий символ (Ctrl+C)
                    //На любой '+' надо ответить '+'. На '-' надо повторить последнее сообщение
                    //Наверно при работе по TCP/IP не будет '-' (поэтому ответ на '-' пока не реализован)
                    rsp_pkt.responce("+");
                }
                else
                {//Пакет
                    rsp_pkt.match_cmd();
                }
            }
            if !rsp_pkt.need_responce.unwrap()
            {//Ответ не требуется. Отдельный if (а не else) т.к. изначальный признак need_responce может быть сброшен в зависимости от команды (только в случае, если это пакет)
                //////////////////////////////Что-то сделать? (Вывести в Лог?)
            }


                //Убрать ======================================================================:
                println!("len of src_packet: {}", rsp_pkt.len.unwrap()); //Длина пакета в буфере
                //println!("Received Buffer: {}", str::from_utf8(&input_buf).unwrap()); //Буфер
                if input_len > 1
                { //Пакет
                    println!("src_packet: {}", &rsp_pkt.src_packet.unwrap()); //Пакет в буфере
                    println!("first_cmd_symbol: {}", &rsp_pkt.first_cmd_symbol.unwrap());
                    println!("data: {}", &rsp_pkt.data.unwrap());
                    println!("cs: {}", &rsp_pkt.cs.unwrap());
                    //println!("responce: {}", &(rsp_pkt.responce.unwrap()));//
                }
                else if input_len == 1
                { //acknowledgment, не пакет
                    println!("only_ack: {}", &rsp_pkt.only_symb.unwrap());
                }
                else //input_len == 0
                {
                }
                if rsp_pkt.need_responce.unwrap()
                {
                    let r = match rsp_pkt.responce //Сделано так, чтобы не было ошибки перемещения
                    {
                        Some(ref v) => v,
                        None => panic!("RspPacket.responce = None"),
                    };
                    println!("responce: {}", &r);

                    if rsp_pkt.output_text.is_some()
                    {
                        let r = match rsp_pkt.output_text
                        {
                            Some(ref v) => v,
                            None => panic!("RspPacket.output_text = None"),
                        };
                        println!("output_text: {}", &r);
                    }
                }
                println!("==================================================\n");


            if rsp_pkt.need_responce.unwrap()
            {//Ответ требуется
                if rsp_pkt.output_text.is_some() //output_text обязательно перед responce
                {//output_text может быть только в ответ на vCont и qRcmd
                    stream.write(&rsp_pkt.output_text.unwrap().as_bytes()).unwrap();
                }
                stream.write(&rsp_pkt.responce.unwrap().as_bytes()).unwrap(); //Ответ в TcpStream. Сделано в конце, чтобы не было ошибки перемещения
            }
            if rsp_pkt.kill_flag.unwrap()
            {
                break;
            }
        }//loop
        break; //kill_flag

    }
    drop(listener);
    println!("Connection was killed!\n"); //Можно подключаться снова
}


///Тесты для RspPacket ================================================================================
#[cfg(test)]
mod test_rsp_packet
{
    use super::*;

    #[test]
    fn test_responce_add_usd_cs()
    {
        ///Создание экземпляра
        let mut input_buf = [0x7Eu8; BUF_SIZE];
        input_buf[0] = b'+';
        let input_len = 1;
        let mut pkt = RspPacket::new(&input_buf, input_len);

        ///Передается строковый литерал
        pkt.responce_add_usd_cs("literal");
        assert_eq!(Some("$literal#ed".to_string()), pkt.responce, "Передается строковый литерал");

        ///Передается ссылка на строку &String (строковый срез)
        pkt.responce_add_usd_cs(&"string".to_string());
        assert_eq!(Some("$string#97".to_string()), pkt.responce, "Передается ссылка на строку &String");

        ///Еще литерал (OK : стандартный ответ)
        pkt.responce_add_usd_cs("OK");
        assert_eq!(Some("$OK#9a".to_string()), pkt.responce);

        //Литерал: пустое сообщение
        pkt.responce_add_usd_cs("");
        assert_eq!(Some("$#00".to_string()), pkt.responce);

        //Тест на панику
        //s : cтрока размера PACKET_SIZE
        //let r = std::panic::catch_unwind(|| { //Так почему-то не работает!! Проверяется отдельным тестом с помощью #[should_panic]
        //error[E0277]: the trait bound `&mut RspPacket: std::panic::UnwindSafe` is not satisfied in `[closure@src\main.rs: pkt:&mut RspPacket, s:&std::string::String]`
        //    pkt.responce_add_usd_cs(&s); //Паники не должно быть
        //});
        //assert!(r.is_ok());

        //Сбросить
        let mut input_buf = [0x7Eu8; BUF_SIZE];
        input_buf[0] = b'+';
        let input_len = 1;
        let pkt = RspPacket::new(&input_buf, input_len);
        //После переинициализации
        assert_eq!(None, pkt.responce);
    }


    #[test]
    #[should_panic] //Тут желательно указать [should_panic(expected = "вид_паники")], но какое у него название?
    fn test_panic_responce_add_usd_cs()
    {
        ///Создание экземпляра
        let mut input_buf = [0x7Eu8; BUF_SIZE];
        input_buf[0] = b'+';
        let input_len = 1;
        let mut pkt = RspPacket::new(&input_buf, input_len);

        ///Строка размера PACKET_SIZE
        let mut s = String::new();
        loop
        {
            s.push('p');
            if s.len() == (PACKET_SIZE - 4) //Строка еще будет дополнена $ и #cs
            {
                break;
            }
        }
        s.push('p'); //s.len() > (PACKET_SIZE - 4)
        pkt.responce_add_usd_cs(&s); //Паника
    }


    #[test]
    fn test_responce()
    {
        ///Создание экземпляра
        let mut input_buf = [0x7Eu8; BUF_SIZE];
        input_buf[0] = b'+';
        let input_len = 1;
        let mut pkt = RspPacket::new(&input_buf, input_len);

        ///Передается строковый литерал
        pkt.responce("literal");
        assert_eq!(Some("literal".to_string()), pkt.responce, "Передается строковый литерал");

        ///Передается ссылка на строку &String (строковый срез)
        pkt.responce(&"string".to_string());
        assert_eq!(Some("string".to_string()), pkt.responce, "Передается ссылка на строку &String");

        ///Еще литерал (+ : стандартный Acknowledgment)
        pkt.responce("+");
        assert_eq!(Some("+".to_string()), pkt.responce);

        ///Еще литерал (OK : стандартный ответ)
        pkt.responce("$OK#9a");
        assert_eq!(Some("$OK#9a".to_string()), pkt.responce);

        //Литерал: пустое сообщение
        pkt.responce("");
        assert_eq!(Some("".to_string()), pkt.responce);

        //Сбросить
        let mut input_buf = [0x7Eu8; BUF_SIZE];
        input_buf[0] = b'+';
        let input_len = 1;
        let pkt = RspPacket::new(&input_buf, input_len);
        //После переинициализации
        assert_eq!(None, pkt.responce);
    }


    #[test]
    #[should_panic] //Тут желательно указать [should_panic(expected = "вид_паники")], но какое у него название?
    fn test_panic_responce()
    {
        ///Создание экземпляра
        let mut input_buf = [0x7Eu8; BUF_SIZE];
        input_buf[0] = b'+';
        let input_len = 1;
        let mut pkt = RspPacket::new(&input_buf, input_len);

        ///Строка размера PACKET_SIZE
        let mut s = String::new();
        loop
        {
            s.push('p');
            if s.len() == PACKET_SIZE
            {
                break;
            }
        }
        s.push('p'); //s.len() > PACKET_SIZE
        pkt.responce(&s); //Паника
    }
}
