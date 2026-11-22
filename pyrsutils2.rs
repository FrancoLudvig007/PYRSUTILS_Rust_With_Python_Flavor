//! # pyrsutils — v2.0.0
//!
//! Biblioteca única de utilidades (**zero dependências externas, 100% `std` estável**)
//! que traz para o Rust a produtividade e a ergonomia do dia a dia de Python (além da
//! sintaxe familiar de classes estilo C++/Python), mantendo integralmente as garantias
//! da linguagem Rust: tipagem estática, checagem em tempo de compilação, ownership,
//! borrowing, lifetimes e *monomorphization* sem *dynamic dispatch* (`dyn Any`) oculto.
//!
//! ## Principais evoluções da versão 2.0.0 (sobre a 1.5.1)
//!
//! - **Compatibilidade preservada**: todas as APIs públicas e testes da v1.5.1 foram
//!   mantidos (RN-03 / RN-04).
//! - **Correções críticas de auditoria**:
//!   - [`range_utils::PyRange`]: proteção contra overflow em `next`/`contains`,
//!     `size_hint` compatível com 32-bit (`usize::try_from`) e suporte completo a
//!     `DoubleEndedIterator` + `FusedIterator`.
//!   - [`matrix_utils::Matrix`]: validação com `checked_mul` + `try_new` (`try_reserve_exact`),
//!     `transpose()` em passagem única (sem dupla clonagem) e remoção de bounds `Clone`
//!     desnecessários em leituras.
//!   - [`builtins`]: `min`/`max` consistentes na presença de `NaN`, variantes explícitas
//!     `try_min`/`try_max`/`try_sorted`/`sorted_f64_total` e pré-alocação protegida em `histogram`.
//!   - [`arithmetic`]: proteção explícita contra overflow em `div`/`divmod` (`i64::MIN / -1`),
//!     `root` corrigida para expoentes grandes/NaN/infinitos/zero negativo, além de `PyInt`,
//!     `PyFloat`, `checked_*`, `saturating_*`, `wrapping_*`, `floor_div`, `py_divmod`, `gcd` e `lcm`.
//!   - [`time_utils`]: `gmtime_now` sem perda de precisão por `f64`, calendário em `i128`,
//!     proteção de índices em `strftime`, `try_mktime_utc` com validação estrita de data,
//!     `Timer` e `Deadline`.
//!   - [`fs_utils`] & [`io_utils`]: `glob` iterativo $O(N \cdot M)$ (imune a backtracking
//!     exponencial), `readlines` via `BufReader`, leitura lazy de linhas, limites de tamanho,
//!     escrita atômica por arquivo temporário no mesmo diretório, `copy_tree` seguro contra symlinks.
//!   - [`net_utils`]: checksum RFC 1071 em `u64`, `ipv4_tcp_checksum` sem alocação intermediária,
//!     validação de `total_len` e padding Ethernet no `Ipv4TcpPacket::parse`, suporte a IPv6 CIDR
//!     (`Ipv6Cidr`), `Ipv4UdpPacket`, `StreamingPcapWriter`/`PcapReader`, opções TCP/UDP avançadas
//!     e scanner com teto seguro de threads e rate limiting.
//!   - [`random_utils`]: remoção de viés de módulo por *rejection sampling*, `PyRng` independente
//!     (`Xoshiro256**` com expansão `SplitMix64`) claramente separado de aleatoriedade criptográfica
//!     (`os_random_bytes` usando `/dev/urandom` em Unix).
//!   - [`sec_utils`]: gerador De Bruijn verdadeiro ($B(26,4)$ via algoritmo FKM), busca streaming
//!     em `cyclic_find`, `base64_decode` RFC 4648 estrito, `BinReader`/`BinWriter` genéricos,
//!     packing/unpacking assinado e de ponto flutuante, `crc32`/`adler32`/`fnv1a_64` e busca AOB.
//!   - [`sorted_utils`]: `nth`/`nth_item` otimizados para percorrer da ponta mais próxima,
//!     `irange_iter` lazy sem alocar `Vec`, e complexidade documentada de `bisect_*`.
//! - **Novos módulos**:
//!   - [`input_utils`]: leitura segura/limitada de linhas e bytes, detecção de TTY e `stdin`/`stdout`/`stderr`.
//!   - [`collection_utils`]: `PyDict`, `PySet`, `PyDeque`, `MinHeap`, `PriorityQueue`.
//!   - [`iterator_utils`]: combinadores lazy, `zip_longest`, `islice`, `batched`, `count_from`, `PyIterExt`.
//!   - [`env_utils`]: camada ergonômica sobre `std::env` preservando `OsString` e documentando efeitos globais.
//!   - [`process_utils`]: wrappers seguros sobre `std::process` (`Command::args`, pipelines, `ManagedChild`).
//!   - [`terminal_utils`]: TTY, cores ANSI configuráveis, prompts, menus, barra de progresso.
//!   - [`concurrency_utils`]: `parallel_map`, `parallel_for_each`, `Shared<T>`, `RateLimiter`, `ThreadPool`.
//!   - [`sql_utils`]: composição parametrizada agnóstica de driver (`SqlBuilder`, `SqlTemplate`, `SqlIdent`).
//!
//! ## Transparência técnica e limitações da `std`
//!
//! - Esta biblioteca não depende de crates externas e utiliza somente APIs estáveis da `std`.
//! - Sem FFI externo, não há acesso a raw sockets do kernel, modo raw de terminal, senha mascarada
//!   portátil, drivers SQL de rede, BigInt arbitrário, segmentação de graphemes Unicode ou base IANA
//!   de fusos horários/horário de verão. Todas essas fronteiras são documentadas onde se aplicam.
//!
//! ## Uso rápido
//! ```rust
//! use pyrsutils2::prelude::*;
//! ```

#![allow(unused)]

use std::fmt::{Debug, Display};
use std::io::{self, Write};
use std::str::FromStr;

// ============================================================================
// 0) input() / try_input() e input_utils — Entrada e Saída Padrão
// ============================================================================

/// Erro unificado de leitura e conversão de entrada padrão (`stdin`) ou leitores `BufRead`.
#[derive(Debug)]
pub enum InputError<E> {
    /// Stream fechado antes de qualquer dado (Ctrl+D no Linux / Ctrl+Z no Windows).
    Eof,
    /// Falha de I/O ao ler entrada ou descarregar `stdout`, ou linha acima do limite configurado.
    Io(io::Error),
    /// O texto lido não pôde ser convertido para o tipo `T` esperado.
    Parse(E),
}

impl<E: Display> Display for InputError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::Eof => write!(f, "EOFError: fim da entrada (Ctrl+D/Ctrl+Z)"),
            InputError::Io(e) => write!(f, "IOError: falha ao ler entrada ({})", e),
            InputError::Parse(e) => write!(f, "ValueError: conversão falhou ({})", e),
        }
    }
}

impl<E: Debug + Display> std::error::Error for InputError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InputError::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Lê uma linha crua de `stdin` removendo apenas `\n` ou `\r\n` final.
fn read_raw(prompt: &str) -> Result<String, io::Error> {
    if !prompt.is_empty() {
        print!("{}", prompt);
        io::stdout().flush()?;
    }

    let mut buffer = String::new();
    let bytes_read = io::stdin().read_line(&mut buffer)?;

    if bytes_read == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
    }

    if buffer.ends_with('\n') {
        buffer.pop();
        if buffer.ends_with('\r') {
            buffer.pop();
        }
    }

    Ok(buffer)
}

/// Converte uma linha em `T` tentando primeiro o texto exato (para preservar espaços
/// quando `T = String`) e, se falhar e houver espaços nas bordas, aplicando `.trim()`.
fn parse_line_py<T: FromStr>(texto: &str) -> Result<T, InputError<T::Err>> {
    match texto.parse::<T>() {
        Ok(v) => Ok(v),
        Err(e) => {
            let aparado = texto.trim();
            if aparado.len() == texto.len() {
                Err(InputError::Parse(e))
            } else {
                aparado.parse::<T>().map_err(InputError::Parse)
            }
        }
    }
}

/// Equivalente unificado ao `input()` (e `int(input())`, `float(input())`) do Python.
///
/// Exibe `prompt` em `stdout` (com `flush`), lê uma linha de `stdin` sem o `\n`/`\r\n` final
/// e converte para `T: FromStr`. Para tipos diferentes de `String`, se a conversão exata falhar
/// por espaços nas extremidades (ex.: `" 42 "`), aplica `.trim()` automaticamente.
///
/// # Panics
/// Encerra com `panic!` em caso de EOF, erro de I/O ou falha de conversão (equivalente a um script
/// Python sem `try/except`). Para tratamento sem panic, utilize [`try_input`] ou
/// [`input_utils::try_input_limited`].
pub fn input<T>(prompt: &str) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    match try_input::<T>(prompt) {
        Ok(valor) => valor,
        Err(InputError::Eof) => panic!("EOFError: entrada finalizada (Ctrl+D/Ctrl+Z)"),
        Err(InputError::Io(e)) => panic!("IOError: erro ao ler stdin: {}", e),
        Err(InputError::Parse(e)) => {
            panic!("ValueError: entrada inválida para o tipo esperado ({:?})", e)
        }
    }
}

/// Versão segura de [`input`] que devolve `Result<T, InputError<T::Err>>` em vez de
/// causar `panic!`.
///
/// Nota: lê uma linha completa de `stdin`. Para entradas potencialmente não confiáveis ou muito
/// grandes, prefira [`input_utils::try_input_limited`].
pub fn try_input<T>(prompt: &str) -> Result<T, InputError<T::Err>>
where
    T: FromStr,
{
    let texto = read_raw(prompt).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            InputError::Eof
        } else {
            InputError::Io(e)
        }
    })?;
    parse_line_py(&texto)
}

/// Macro ergonômica: `input!()` ou `input!("Digite: ")`.
/// Usa `$crate::input` para funcionar em qualquer módulo sem imports extras.
#[macro_export]
macro_rules! input {
    () => {
        $crate::input("")
    };
    ($prompt:expr) => {
        $crate::input($prompt)
    };
}

/// Macro ergonômica: `try_input!()` ou `try_input!("Digite: ")`.
#[macro_export]
macro_rules! try_input {
    () => {
        $crate::try_input("")
    };
    ($prompt:expr) => {
        $crate::try_input($prompt)
    };
}

pub mod input_utils {
    //! Utilitários ergonômicos e seguros para `stdin`, `stdout`, `stderr` e leitores `BufRead`.
    //!
    //! Fornece leitura de linhas com tratamento explícito de EOF (`Ok(None)`), leitura limitada em
    //! bytes contra entrada maliciosa/redirecionada sem fim, escrita direta em bytes e detecção de
    //! terminal interativo (`IsTerminal`).
    //!
    //! **Limitação da `std`:** entrada de senha mascarada (desabilitar echo do terminal) não é
    //! portável usando exclusivamente a Rust Standard Library estável sem chamadas específicas de OS/FFI.

    pub use super::{input, try_input, InputError};

    use std::fmt::Display;
    use std::io::{self, BufRead, IsTerminal, Read, Write};
    use std::str::FromStr;

    /// Lê uma única linha de um leitor `BufRead` respeitando o limite máximo de `max_bytes` bytes
    /// úteis (excluindo terminador `\n`/`\r\n`).
    ///
    /// - Retorna `Ok(None)` se o stream já estiver em EOF antes de ler qualquer byte.
    /// - Retorna `Err(InvalidData)` se a linha ultrapassar `max_bytes` ou não for UTF-8 válido.
    /// - Preserva o restante do stream sem consumir além do `\n`.
    pub fn read_line_from_limited<R: BufRead>(
        reader: &mut R,
        max_bytes: usize,
    ) -> io::Result<Option<String>> {
        let mut buf: Vec<u8> = Vec::new();
        let mut leu_algo = false;

        loop {
            let chunk = reader.fill_buf()?;
            if chunk.is_empty() {
                if !leu_algo {
                    return Ok(None);
                }
                break;
            }
            leu_algo = true;

            if let Some(pos_nl) = chunk.iter().position(|&b| b == b'\n') {
                let antes_nl = &chunk[..pos_nl];
                let sem_cr = antes_nl.strip_suffix(b"\r").unwrap_or(antes_nl);
                let novo_tam = buf.len().saturating_add(sem_cr.len());
                if novo_tam > max_bytes {
                    reader.consume(pos_nl + 1);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("linha excedeu o limite máximo de {} bytes", max_bytes),
                    ));
                }
                let antes_vazio = antes_nl.is_empty();
                buf.extend_from_slice(sem_cr);
                reader.consume(pos_nl + 1);
                // Remove um possível '\r' que tenha ficado na borda do chunk anterior
                if antes_vazio && buf.ends_with(b"\r") {
                    buf.pop();
                }
                break;
            } else {
                // Cuidado para caso o último byte do chunk seja '\r' e o próximo seja '\n':
                // permitimos temporariamente até +1 byte se o último byte for '\r'.
                let folga = if chunk.last() == Some(&b'\r') { 1 } else { 0 };
                if buf.len().saturating_add(chunk.len()) > max_bytes.saturating_add(folga) {
                    let n = chunk.len();
                    reader.consume(n);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("linha excedeu o limite máximo de {} bytes", max_bytes),
                    ));
                }
                buf.extend_from_slice(chunk);
                let n = chunk.len();
                reader.consume(n);
            }
        }

        if buf.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("linha excedeu o limite máximo de {} bytes", max_bytes),
            ));
        }

        String::from_utf8(buf).map(Some).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("UTF-8 inválido na entrada: {}", e),
            )
        })
    }

    /// Lê uma linha de qualquer `BufRead` e converte para `T: FromStr` (sem prompt).
    pub fn try_input_from<R: BufRead, T: FromStr>(
        reader: &mut R,
    ) -> Result<T, InputError<T::Err>> {
        let mut linha = String::new();
        let n = reader.read_line(&mut linha).map_err(InputError::Io)?;
        if n == 0 {
            return Err(InputError::Eof);
        }
        if linha.ends_with('\n') {
            linha.pop();
            if linha.ends_with('\r') {
                linha.pop();
            }
        }
        super::parse_line_py(&linha)
    }

    /// Exibe `prompt`, lê uma linha com limite máximo de `max_bytes` bytes e converte para `T`.
    /// Protege contra alocação irrestrita de memória caso `stdin` esteja redirecionado para um
    /// fluxo infinito ou arquivo gigante sem quebras de linha.
    pub fn try_input_limited<T: FromStr>(
        prompt: &str,
        max_bytes: usize,
    ) -> Result<T, InputError<T::Err>> {
        if !prompt.is_empty() {
            print!("{}", prompt);
            io::stdout().flush().map_err(InputError::Io)?;
        }
        let stdin = io::stdin();
        let mut lock = stdin.lock();
        match read_line_from_limited(&mut lock, max_bytes) {
            Ok(Some(texto)) => super::parse_line_py(&texto),
            Ok(None) => Err(InputError::Eof),
            Err(e) => Err(InputError::Io(e)),
        }
    }

    /// Tenta ler e converter uma entrada de `stdin`; se ocorrer EOF, falha de I/O ou erro de
    /// conversão, retorna `default` sem causar panic.
    pub fn input_or<T: FromStr>(prompt: &str, default: T) -> T {
        try_input::<T>(prompt).unwrap_or(default)
    }

    /// Lê uma linha de `stdin` removendo `\n`/`\r\n`. Retorna `Ok(None)` em EOF.
    pub fn read_line() -> io::Result<Option<String>> {
        let mut buf = String::new();
        let n = io::stdin().read_line(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }
        if buf.ends_with('\n') {
            buf.pop();
            if buf.ends_with('\r') {
                buf.pop();
            }
        }
        Ok(Some(buf))
    }

    /// Lê uma linha de `stdin` com limite explícito de `max_bytes`. Retorna `Ok(None)` em EOF.
    pub fn read_line_limited(max_bytes: usize) -> io::Result<Option<String>> {
        let stdin = io::stdin();
        let mut lock = stdin.lock();
        read_line_from_limited(&mut lock, max_bytes)
    }

    /// Retorna um iterador lazy sobre as linhas de `stdin` (`for linha in read_stdin_lines()`).
    #[inline]
    pub fn read_stdin_lines() -> io::Lines<io::StdinLock<'static>> {
        io::stdin().lines()
    }

    /// Lê no máximo `max_bytes` bytes brutos de `stdin` (útil para pipelines binários).
    pub fn read_stdin_bytes(max_bytes: usize) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        io::stdin()
            .take(max_bytes as u64)
            .read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Escreve uma fatia de bytes diretamente em `stdout` e executa `flush()`.
    pub fn write_stdout_bytes(dados: &[u8]) -> io::Result<()> {
        let mut out = io::stdout().lock();
        out.write_all(dados)?;
        out.flush()
    }

    /// Escreve uma fatia de bytes diretamente em `stderr` e executa `flush()`.
    pub fn write_stderr_bytes(dados: &[u8]) -> io::Result<()> {
        let mut err = io::stderr().lock();
        err.write_all(dados)?;
        err.flush()
    }

    /// Imprime um valor `Display` em `stderr` seguido de quebra de linha.
    #[inline]
    pub fn eprint_line<T: Display>(valor: T) {
        eprintln!("{}", valor);
    }

    /// Indica se `stdin` está conectado a um terminal interativo (TTY) e não a pipe/arquivo.
    #[inline]
    pub fn stdin_is_tty() -> bool {
        io::stdin().is_terminal()
    }

    /// Indica se `stdout` está conectado a um terminal interativo (TTY).
    #[inline]
    pub fn stdout_is_tty() -> bool {
        io::stdout().is_terminal()
    }

    /// Indica se `stderr` está conectado a um terminal interativo (TTY).
    #[inline]
    pub fn stderr_is_tty() -> bool {
        io::stderr().is_terminal()
    }
}

// ============================================================================
// 1) range() — sequências lazy sem alocar memória (Iterator O(1) em espaço)
// ============================================================================
pub mod range_utils {
    //! Equivalente ao `range()` do Python para inteiros com sinal (`i64`) e índices (`usize`).
    //!
    //! | Python                     | Aqui                                |
    //! |----------------------------|-------------------------------------|
    //! | `range(stop)`              | `range(stop)` / `range_idx(stop)`   |
    //! | `range(start, stop)`       | `range_from(start, stop)`           |
    //! | `range(start, stop, step)` | `range_step` / `try_range_step`     |

    use super::builtins::PyLen;
    use std::convert::TryFrom;
    use std::fmt;
    use std::iter::FusedIterator;

    /// Erro ao construir um intervalo com passo inválido.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RangeError {
        /// O parâmetro `step` foi igual a zero.
        StepZero,
    }

    impl fmt::Display for RangeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                RangeError::StepZero => write!(f, "ValueError: range() com step igual a zero"),
            }
        }
    }

    impl std::error::Error for RangeError {}

    /// Iterador preguiçoso equivalente ao `range` do Python (`i64`, com suporte a passo
    /// positivo ou negativo, `DoubleEndedIterator`, `ExactSizeIterator` e `FusedIterator`).
    ///
    /// Na v2.0.0, todos os cálculos de limites, `contains`, `next` e `next_back` utilizam
    /// aritmética protegida (`i128` / `checked_add`), evitando overflows em `i64::MIN`/`i64::MAX`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PyRange {
        atual: i64,
        fim: i64,
        passo: i64,
    }

    impl PyRange {
        /// Primeiro elemento potencial do intervalo (`start`).
        #[inline]
        pub const fn start(&self) -> i64 {
            self.atual
        }

        /// Limite exclusivo do intervalo (`stop`).
        #[inline]
        pub const fn stop(&self) -> i64 {
            self.fim
        }

        /// Passo do intervalo (`step`).
        #[inline]
        pub const fn step(&self) -> i64 {
            self.passo
        }

        /// Quantidade exata de elementos restantes como `u128` (sem risco de truncamento).
        #[inline]
        pub fn len_u128(&self) -> u128 {
            let atual = self.atual as i128;
            let fim = self.fim as i128;
            let passo = self.passo as i128;
            let n = if passo > 0 {
                if atual >= fim {
                    0
                } else {
                    (fim - atual + passo - 1) / passo
                }
            } else if atual <= fim {
                0
            } else {
                let passo_abs = -passo;
                (atual - fim + passo_abs - 1) / passo_abs
            };
            n as u128
        }

        /// Retorna `true` se o intervalo não contiver elementos restantes.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.len_u128() == 0
        }

        /// Verifica em $O(1)$ se `x` pertence à sequência (`x in range(...)`), livre de overflow.
        #[inline]
        pub fn contains(&self, x: i64) -> bool {
            let xi = x as i128;
            let atual = self.atual as i128;
            let fim = self.fim as i128;
            let passo = self.passo as i128;
            if passo > 0 {
                xi >= atual && xi < fim && (xi - atual) % passo == 0
            } else {
                xi <= atual && xi > fim && (atual - xi) % (-passo) == 0
            }
        }

        /// Retorna o `index`-ésimo elemento em $O(1)$ sem iterar (`r[i]`).
        #[inline]
        pub fn get(&self, index: usize) -> Option<i64> {
            if (index as u128) >= self.len_u128() {
                return None;
            }
            let val = (self.atual as i128) + (index as i128) * (self.passo as i128);
            Some(val as i64)
        }

        /// Retorna a posição de `x` dentro do intervalo em $O(1)$ (`r.index(x)`), ou `None`.
        #[inline]
        pub fn index_of(&self, x: i64) -> Option<usize> {
            if !self.contains(x) {
                return None;
            }
            let idx = (x as i128 - self.atual as i128) / (self.passo as i128);
            usize::try_from(idx).ok()
        }
    }

    impl Iterator for PyRange {
        type Item = i64;

        #[inline]
        fn next(&mut self) -> Option<i64> {
            if self.passo > 0 {
                if self.atual >= self.fim {
                    return None;
                }
            } else if self.atual <= self.fim {
                return None;
            }
            let valor = self.atual;
            match self.atual.checked_add(self.passo) {
                Some(prox) => self.atual = prox,
                None => self.atual = self.fim,
            }
            Some(valor)
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let n = self.len_u128();
            match usize::try_from(n) {
                Ok(v) => (v, Some(v)),
                Err(_) => (usize::MAX, None),
            }
        }
    }

    impl DoubleEndedIterator for PyRange {
        #[inline]
        fn next_back(&mut self) -> Option<i64> {
            let n = self.len_u128();
            if n == 0 {
                return None;
            }
            let ultimo = ((self.atual as i128) + ((n - 1) as i128) * (self.passo as i128)) as i64;
            self.fim = ultimo;
            Some(ultimo)
        }
    }

    impl ExactSizeIterator for PyRange {
        #[inline]
        fn len(&self) -> usize {
            usize::try_from(self.len_u128()).unwrap_or(usize::MAX)
        }
    }

    impl FusedIterator for PyRange {}

    impl PyLen for PyRange {
        #[inline]
        fn py_len(&self) -> usize {
            ExactSizeIterator::len(self)
        }
    }

    /// `range(stop)` — gera `0, 1, ..., stop - 1` (vazio se `stop <= 0`).
    #[inline]
    pub fn range(stop: i64) -> PyRange {
        PyRange {
            atual: 0,
            fim: stop,
            passo: 1,
        }
    }

    /// `range_from(start, stop)` — gera `start, start + 1, ..., stop - 1`.
    #[inline]
    pub fn range_from(start: i64, stop: i64) -> PyRange {
        PyRange {
            atual: start,
            fim: stop,
            passo: 1,
        }
    }

    /// `range_step(start, stop, step)` — suporta passo positivo ou negativo.
    ///
    /// # Panics
    /// Causa panic se `step == 0` (`ValueError` no Python). Para construção sem panic, use
    /// [`try_range_step`].
    #[inline]
    pub fn range_step(start: i64, stop: i64, step: i64) -> PyRange {
        assert!(step != 0, "ValueError: range() com step igual a zero");
        PyRange {
            atual: start,
            fim: stop,
            passo: step,
        }
    }

    /// Versão segura de [`range_step`] que retorna `Err(RangeError::StepZero)` quando `step == 0`.
    #[inline]
    pub fn try_range_step(start: i64, stop: i64, step: i64) -> Result<PyRange, RangeError> {
        if step == 0 {
            Err(RangeError::StepZero)
        } else {
            Ok(PyRange {
                atual: start,
                fim: stop,
                passo: step,
            })
        }
    }

    /// `range_idx(stop)` — gera índices `0..stop` do tipo `usize` diretamente para indexação de slices.
    #[inline]
    pub fn range_idx(stop: usize) -> std::ops::Range<usize> {
        0..stop
    }

    /// `range_idx_from(start, stop)` — gera índices `start..stop` do tipo `usize` (vazio se `start >= stop`).
    #[inline]
    pub fn range_idx_from(start: usize, stop: usize) -> std::ops::Range<usize> {
        if start >= stop {
            stop..stop
        } else {
            start..stop
        }
    }
}

// ============================================================================
// 2) Listas — métodos estilo Python sobre Vec<T>, via Extension Trait
// ============================================================================
pub mod list_utils {
    //! Métodos ergonômicos estilo Python sobre `Vec<T>` sem dispatch dinâmico.
    //!
    //! Preserva todos os métodos da v1.5.1 e adiciona suporte a indexação negativa (`get_py`,
    //! `pop_py`, `insert_py`), slicing zero-copy (`slice_py`), slicing com passo (`slice_step`),
    //! deduplicação preservando ordem, remoção em $O(1)$ com troca (`swap_remove_value`),
    //! remoção de todas as ocorrências (`remove_all`) e ordenações auxiliares.

    use std::collections::HashSet;
    use std::hash::Hash;

    /// Converte índice estilo Python (`-1` = último) para índice válido `usize`, se dentro dos limites.
    #[inline]
    fn resolve_index(len: usize, index: isize) -> Option<usize> {
        let pos = if index < 0 {
            (len as i128) + (index as i128)
        } else {
            index as i128
        };
        if pos >= 0 && pos < (len as i128) {
            Some(pos as usize)
        } else {
            None
        }
    }

    /// Ajusta um índice de corte estilo Python (`start`/`end` de `v[start:end]`) para `0..=len`.
    #[inline]
    fn clamp_slice_bound(len: usize, bound: isize) -> usize {
        let len_i = len as i128;
        let b = if bound < 0 {
            len_i + (bound as i128)
        } else {
            bound as i128
        };
        if b < 0 {
            0
        } else if b > len_i {
            len
        } else {
            b as usize
        }
    }

    /// Extension trait que adiciona ergonomia estilo Python (`list`) sobre `Vec<T>`.
    pub trait PyList<T> {
        /// Adiciona um item ao final — equivalente a `lista.append(x)` ($O(1)$ amortizado).
        fn append_item(&mut self, valor: T);

        /// Estende a lista com qualquer iterável — equivalente a `lista.extend(iter)`.
        fn extend_from<I: IntoIterator<Item = T>>(&mut self, iter: I);

        /// Remove e devolve o item no índice dado, ou o último se `None` —
        /// equivalente a `lista.pop()` / `lista.pop(i)`.
        ///
        /// Nota de complexidade: `None` é $O(1)$; `Some(i)` desloca elementos em $O(N - i)$.
        fn pop_at(&mut self, indice: Option<usize>) -> Option<T>;

        /// Remove a primeira ocorrência igual a `valor` — equivalente a `lista.remove(x)`.
        /// Devolve `true` se encontrou e removeu. Complexidade: $O(N)$.
        fn remove_value(&mut self, valor: &T) -> bool
        where
            T: PartialEq;

        /// Devolve o índice da primeira ocorrência de `valor` — equivalente a `lista.index(x)`.
        fn index_of(&self, valor: &T) -> Option<usize>
        where
            T: PartialEq;

        /// Conta quantas vezes `valor` aparece na lista — equivalente a `lista.count(x)`.
        fn count_value(&self, valor: &T) -> usize
        where
            T: PartialEq;

        /// Insere `valor` na posição estilo Python `index` (suporta negativos; valores fora dos
        /// limites são ajustados para `0` ou `len`), equivalente a `lista.insert(i, x)`.
        fn insert_py(&mut self, index: isize, valor: T);

        /// Remove e devolve o item na posição estilo Python `index` (ex.: `-1` remove o último).
        fn pop_py(&mut self, index: isize) -> Option<T>;

        /// Acessa elemento por índice positivo ou negativo (ex.: `v.get_py(-1)`), sem panic.
        fn get_py(&self, index: isize) -> Option<&T>;

        /// Acesso mutável por índice positivo ou negativo (ex.: `v.get_py_mut(-1)`), sem panic.
        fn get_py_mut(&mut self, index: isize) -> Option<&mut T>;

        /// Fatiamento estilo Python `v[start:end]` com custo zero de alocação (retorna `&[T]`).
        fn slice_py(&self, start: Option<isize>, end: Option<isize>) -> &[T];

        /// Fatiamento com passo estilo Python `v[start:end:step]` (suporta `step < 0`).
        ///
        /// # Panics
        /// Causa panic se `step == 0`.
        fn slice_step(&self, start: Option<isize>, end: Option<isize>, step: isize) -> Vec<T>
        where
            T: Clone;

        /// Remove a primeira ocorrência de `valor` trocando com o último elemento em $O(1)$
        /// após a busca (não preserva a ordem dos elementos restantes).
        fn swap_remove_value(&mut self, valor: &T) -> bool
        where
            T: PartialEq;

        /// Remove todas as ocorrências iguais a `valor` em uma única passagem $O(N)$ e
        /// devolve quantos elementos foram removidos.
        fn remove_all(&mut self, valor: &T) -> usize
        where
            T: PartialEq;

        /// Busca `valor` no subintervalo `[start, end)` limitado ao comprimento da lista.
        fn index_in(&self, valor: &T, start: usize, end: usize) -> Option<usize>
        where
            T: PartialEq;

        /// Remove duplicatas preservando a ordem da primeira ocorrência (`O(N)` médio).
        fn dedup_preserve_order(&mut self)
        where
            T: Eq + Hash + Clone;

        /// Cópia rasa equivalente a `lista.copy()`.
        fn copy_list(&self) -> Vec<T>
        where
            T: Clone;

        /// Ordena em ordem decrescente in-place (`lista.sort(reverse=True)`).
        fn sort_desc(&mut self)
        where
            T: Ord;

        /// Ordena usando função chave (`lista.sort(key=...)`).
        fn sort_by_key_py<K: Ord, F: FnMut(&T) -> K>(&mut self, f: F);
    }

    impl<T> PyList<T> for Vec<T> {
        #[inline]
        fn append_item(&mut self, valor: T) {
            self.push(valor);
        }

        #[inline]
        fn extend_from<I: IntoIterator<Item = T>>(&mut self, iter: I) {
            self.extend(iter);
        }

        #[inline]
        fn pop_at(&mut self, indice: Option<usize>) -> Option<T> {
            match indice {
                None => self.pop(),
                Some(i) if i < self.len() => Some(self.remove(i)),
                Some(_) => None,
            }
        }

        fn remove_value(&mut self, valor: &T) -> bool
        where
            T: PartialEq,
        {
            match self.iter().position(|x| x == valor) {
                Some(pos) => {
                    self.remove(pos);
                    true
                }
                None => false,
            }
        }

        #[inline]
        fn index_of(&self, valor: &T) -> Option<usize>
        where
            T: PartialEq,
        {
            self.iter().position(|x| x == valor)
        }

        #[inline]
        fn count_value(&self, valor: &T) -> usize
        where
            T: PartialEq,
        {
            self.iter().filter(|&x| x == valor).count()
        }

        fn insert_py(&mut self, index: isize, valor: T) {
            let pos = clamp_slice_bound(self.len(), index);
            self.insert(pos, valor);
        }

        #[inline]
        fn pop_py(&mut self, index: isize) -> Option<T> {
            let pos = resolve_index(self.len(), index)?;
            Some(self.remove(pos))
        }

        #[inline]
        fn get_py(&self, index: isize) -> Option<&T> {
            let pos = resolve_index(self.len(), index)?;
            self.get(pos)
        }

        #[inline]
        fn get_py_mut(&mut self, index: isize) -> Option<&mut T> {
            let pos = resolve_index(self.len(), index)?;
            self.get_mut(pos)
        }

        #[inline]
        fn slice_py(&self, start: Option<isize>, end: Option<isize>) -> &[T] {
            let n = self.len();
            let ini = start.map_or(0, |s| clamp_slice_bound(n, s));
            let fim = end.map_or(n, |e| clamp_slice_bound(n, e));
            if ini >= fim {
                &[]
            } else {
                &self[ini..fim]
            }
        }

        fn slice_step(&self, start: Option<isize>, end: Option<isize>, step: isize) -> Vec<T>
        where
            T: Clone,
        {
            assert!(step != 0, "ValueError: slice step cannot be zero");
            let n = self.len();
            if n == 0 {
                return Vec::new();
            }
            let n_i = n as i128;
            let step_i = step as i128;
            let mut out = Vec::new();

            if step_i > 0 {
                let ini = start.map_or(0, |s| clamp_slice_bound(n, s)) as i128;
                let fim = end.map_or(n, |e| clamp_slice_bound(n, e)) as i128;
                let mut i = ini;
                while i < fim {
                    out.push(self[i as usize].clone());
                    i += step_i;
                }
            } else {
                let clamp_rev = |b: isize| -> i128 {
                    let v = if b < 0 { n_i + (b as i128) } else { b as i128 };
                    if v < -1 {
                        -1
                    } else if v >= n_i {
                        n_i - 1
                    } else {
                        v
                    }
                };
                let ini = start.map_or(n_i - 1, clamp_rev);
                let fim = end.map_or(-1, clamp_rev);
                let mut i = ini;
                while i > fim {
                    out.push(self[i as usize].clone());
                    i += step_i;
                }
            }
            out
        }

        fn swap_remove_value(&mut self, valor: &T) -> bool
        where
            T: PartialEq,
        {
            match self.iter().position(|x| x == valor) {
                Some(pos) => {
                    self.swap_remove(pos);
                    true
                }
                None => false,
            }
        }

        fn remove_all(&mut self, valor: &T) -> usize
        where
            T: PartialEq,
        {
            let antes = self.len();
            self.retain(|x| x != valor);
            antes - self.len()
        }

        fn index_in(&self, valor: &T, start: usize, end: usize) -> Option<usize>
        where
            T: PartialEq,
        {
            let n = self.len();
            let s = start.min(n);
            let e = end.min(n);
            if s >= e {
                return None;
            }
            self[s..e].iter().position(|x| x == valor).map(|off| s + off)
        }

        fn dedup_preserve_order(&mut self)
        where
            T: Eq + Hash + Clone,
        {
            let mut vistos = HashSet::with_capacity(self.len());
            self.retain(|item| vistos.insert(item.clone()));
        }

        #[inline]
        fn copy_list(&self) -> Vec<T>
        where
            T: Clone,
        {
            self.clone()
        }

        #[inline]
        fn sort_desc(&mut self)
        where
            T: Ord,
        {
            self.sort_by(|a, b| b.cmp(a));
        }

        #[inline]
        fn sort_by_key_py<K: Ord, F: FnMut(&T) -> K>(&mut self, f: F) {
            self.sort_by_key(f);
        }
    }
}

// ============================================================================
// 2B) Coleções Nativas — HashMap (PyDict), HashSet (PySet), VecDeque (PyDeque),
//     BinaryHeap (MinHeap / PriorityQueue)
// ============================================================================
pub mod collection_utils {
    //! Interfaces ergonômicas estilo Python sobre as coleções da `std`:
    //! `HashMap` ([`PyDict`]), `HashSet` ([`PySet`]), `VecDeque` ([`PyDeque`]) e
    //! filas de prioridade ([`MinHeap`], [`PriorityQueue`]).
    //!
    //! Todas as extensões evitam colisão com os métodos inerentes da `std` (que mantêm prioridade)
    //! e preservam o controle explícito sobre clonagens e empréstimos (`&V` vs `V`).

    use super::builtins::PyLen;
    use std::borrow::Borrow;
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
    use std::fmt::{self, Debug};
    use std::hash::{BuildHasher, Hash};
    use std::iter::FromIterator;

    /// Erro lançado quando uma chave ou elemento obrigatório está ausente (`KeyError` do Python).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct KeyError(pub String);

    impl fmt::Display for KeyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "KeyError: {}", self.0)
        }
    }

    impl std::error::Error for KeyError {}

    /// Métodos Python-like para `HashMap<K, V, S>` sem colidir com a API inerente da `std`.
    pub trait PyDict<K, V> {
        /// `d.get(k, default)` — devolve o valor clonado se presente ou `default`.
        fn get_or<Q>(&self, key: &Q, default: V) -> V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
            V: Clone;

        /// Variante zero-copy de `get_or`: retorna referência emprestada `&V` sem clonar.
        fn get_or_ref<'a, Q>(&'a self, key: &Q, default: &'a V) -> &'a V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized;

        /// `d.get(k)` com fallback preguiçoso `f()` (só executa `f` se a chave estiver ausente).
        fn get_or_else<Q, F>(&self, key: &Q, f: F) -> V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
            V: Clone,
            F: FnOnce() -> V;

        /// `d.setdefault(k, default)` — insere `default` se a chave não existir e devolve `&mut V`.
        fn setdefault(&mut self, key: K, default: V) -> &mut V;

        /// Variante preguiçosa de [`PyDict::setdefault`]: só chama `f()` se a chave não existir.
        fn setdefault_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> &mut V;

        /// `d.pop(k)` — remove a chave e devolve o valor (`None` se ausente).
        fn pop<Q>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized;

        /// `d.pop(k, default)` — remove a chave ou devolve `default`.
        fn pop_or<Q>(&mut self, key: &Q, default: V) -> V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized;

        /// `d.update(iter)` — insere ou sobrescreve pares `(K, V)`.
        fn update<I: IntoIterator<Item = (K, V)>>(&mut self, items: I);

        /// `d.items()` — iterador lazy de pares `(&K, &V)`.
        fn items(&self) -> std::collections::hash_map::Iter<'_, K, V>;

        /// Atalho ergonômico sobre `entry`: modifica o valor existente com `mod_fn` ou insere `default`.
        fn modify_or_insert<F: FnOnce(&mut V)>(&mut self, key: K, default: V, mod_fn: F)
            -> &mut V;

        /// `d1 | d2` — cria um novo `HashMap` contendo os pares de `self` atualizados por `other`.
        fn merge_cloned(&self, other: &Self) -> HashMap<K, V>
        where
            K: Clone + Eq + Hash,
            V: Clone;
    }

    impl<K, V, S> PyDict<K, V> for HashMap<K, V, S>
    where
        K: Eq + Hash,
        S: BuildHasher,
    {
        #[inline]
        fn get_or<Q>(&self, key: &Q, default: V) -> V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
            V: Clone,
        {
            self.get(key).cloned().unwrap_or(default)
        }

        #[inline]
        fn get_or_ref<'a, Q>(&'a self, key: &Q, default: &'a V) -> &'a V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
        {
            self.get(key).unwrap_or(default)
        }

        #[inline]
        fn get_or_else<Q, F>(&self, key: &Q, f: F) -> V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
            V: Clone,
            F: FnOnce() -> V,
        {
            match self.get(key) {
                Some(v) => v.clone(),
                None => f(),
            }
        }

        #[inline]
        fn setdefault(&mut self, key: K, default: V) -> &mut V {
            self.entry(key).or_insert(default)
        }

        #[inline]
        fn setdefault_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> &mut V {
            self.entry(key).or_insert_with(f)
        }

        #[inline]
        fn pop<Q>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
        {
            self.remove(key)
        }

        #[inline]
        fn pop_or<Q>(&mut self, key: &Q, default: V) -> V
        where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
        {
            self.remove(key).unwrap_or(default)
        }

        #[inline]
        fn update<I: IntoIterator<Item = (K, V)>>(&mut self, items: I) {
            self.extend(items);
        }

        #[inline]
        fn items(&self) -> std::collections::hash_map::Iter<'_, K, V> {
            self.iter()
        }

        #[inline]
        fn modify_or_insert<F: FnOnce(&mut V)>(
            &mut self,
            key: K,
            default: V,
            mod_fn: F,
        ) -> &mut V {
            self.entry(key).and_modify(mod_fn).or_insert(default)
        }

        fn merge_cloned(&self, other: &Self) -> HashMap<K, V>
        where
            K: Clone + Eq + Hash,
            V: Clone,
        {
            let mut out = HashMap::with_capacity(self.len().saturating_add(other.len()));
            for (k, v) in self.iter() {
                out.insert(k.clone(), v.clone());
            }
            for (k, v) in other.iter() {
                out.insert(k.clone(), v.clone());
            }
            out
        }
    }

    /// Métodos Python-like para `HashSet<T, S>`.
    pub trait PySet<T> {
        /// `s.add(x)` — insere `value`; devolve `true` se o conjunto não continha o elemento.
        fn add(&mut self, value: T) -> bool;

        /// `s.discard(x)` — remove se presente; não falha se ausente. Devolve `true` se removeu.
        fn discard<Q>(&mut self, value: &Q) -> bool
        where
            T: Borrow<Q>,
            Q: Eq + Hash + ?Sized;

        /// `s.remove(x)` estrito — devolve `Err(KeyError)` se o elemento não existir no conjunto.
        fn remove_strict<Q>(&mut self, value: &Q) -> Result<(), KeyError>
        where
            T: Borrow<Q>,
            Q: Eq + Hash + Debug + ?Sized;

        /// `s.update(iter)` — insere todos os elementos do iterável.
        fn update<I: IntoIterator<Item = T>>(&mut self, items: I);

        /// `a | b` — novo conjunto com a união de `self` e `other`.
        fn union_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash;

        /// `a & b` — novo conjunto com a interseção de `self` e `other`.
        fn intersection_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash;

        /// `a - b` — novo conjunto com os elementos de `self` ausentes em `other`.
        fn difference_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash;

        /// `a ^ b` — novo conjunto com a diferença simétrica entre `self` e `other`.
        fn symmetric_difference_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash;
    }

    impl<T, S> PySet<T> for HashSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        #[inline]
        fn add(&mut self, value: T) -> bool {
            self.insert(value)
        }

        #[inline]
        fn discard<Q>(&mut self, value: &Q) -> bool
        where
            T: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
        {
            self.remove(value)
        }

        fn remove_strict<Q>(&mut self, value: &Q) -> Result<(), KeyError>
        where
            T: Borrow<Q>,
            Q: Eq + Hash + Debug + ?Sized,
        {
            if self.remove(value) {
                Ok(())
            } else {
                Err(KeyError(format!("{:?}", value)))
            }
        }

        #[inline]
        fn update<I: IntoIterator<Item = T>>(&mut self, items: I) {
            self.extend(items);
        }

        fn union_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash,
        {
            let mut out = HashSet::with_capacity(self.len().saturating_add(other.len()));
            for x in self.iter() {
                out.insert(x.clone());
            }
            for x in other.iter() {
                out.insert(x.clone());
            }
            out
        }

        fn intersection_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash,
        {
            let mut out = HashSet::new();
            if self.len() <= other.len() {
                for x in self.iter() {
                    if other.contains(x) {
                        out.insert(x.clone());
                    }
                }
            } else {
                for x in other.iter() {
                    if self.contains(x) {
                        out.insert(x.clone());
                    }
                }
            }
            out
        }

        fn difference_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash,
        {
            self.iter().filter(|x| !other.contains(x)).cloned().collect()
        }

        fn symmetric_difference_set<S2: BuildHasher>(&self, other: &HashSet<T, S2>) -> HashSet<T>
        where
            T: Clone + Eq + Hash,
        {
            let mut out = HashSet::new();
            for x in self.iter() {
                if !other.contains(x) {
                    out.insert(x.clone());
                }
            }
            for x in other.iter() {
                if !self.contains(x) {
                    out.insert(x.clone());
                }
            }
            out
        }
    }

    /// Métodos estilo `collections.deque` do Python sobre `VecDeque<T>`.
    ///
    /// Os nomes evitam colisão com os métodos inerentes de `VecDeque` (`append`, `push_front`,
    /// `push_back`, `pop_front`, `pop_back`, `rotate_left`, `rotate_right`).
    pub trait PyDeque<T> {
        /// `dq.append(x)` — insere no final direito em $O(1)$ amortizado.
        fn append_item(&mut self, valor: T);
        /// `dq.appendleft(x)` — insere no início esquerdo em $O(1)$ amortizado.
        fn appendleft(&mut self, valor: T);
        /// `dq.pop()` — remove e devolve o elemento da direita em $O(1)$.
        fn pop_right(&mut self) -> Option<T>;
        /// `dq.popleft()` — remove e devolve o elemento da esquerda em $O(1)$.
        fn popleft(&mut self) -> Option<T>;
        /// `dq.extendleft(iter)` — insere cada item na frente (em Python, reverte a ordem).
        fn extend_left<I: IntoIterator<Item = T>>(&mut self, iter: I);
        /// `dq.rotate(n)` — rotaciona `n` passos à direita se `n > 0` ou à esquerda se `n < 0`.
        fn rotate_py(&mut self, n: isize);
        /// Espia o elemento esquerdo sem remover.
        fn peek_left(&self) -> Option<&T>;
        /// Espia o elemento direito sem remover.
        fn peek_right(&self) -> Option<&T>;
    }

    impl<T> PyDeque<T> for VecDeque<T> {
        #[inline]
        fn append_item(&mut self, valor: T) {
            self.push_back(valor);
        }

        #[inline]
        fn appendleft(&mut self, valor: T) {
            self.push_front(valor);
        }

        #[inline]
        fn pop_right(&mut self) -> Option<T> {
            self.pop_back()
        }

        #[inline]
        fn popleft(&mut self) -> Option<T> {
            self.pop_front()
        }

        fn extend_left<I: IntoIterator<Item = T>>(&mut self, iter: I) {
            for item in iter {
                self.push_front(item);
            }
        }

        fn rotate_py(&mut self, n: isize) {
            let len = self.len();
            if len <= 1 || n == 0 {
                return;
            }
            let shift = (n as i128).rem_euclid(len as i128) as usize;
            if shift > 0 {
                self.rotate_right(shift);
            }
        }

        #[inline]
        fn peek_left(&self) -> Option<&T> {
            self.front()
        }

        #[inline]
        fn peek_right(&self) -> Option<&T> {
            self.back()
        }
    }

    /// Min-Heap ergonômica equivalente ao `heapq` do Python (`BinaryHeap<Reverse<T>>`).
    ///
    /// Sempre extrai o **menor** elemento em $O(\log N)$ e inspeciona o mínimo em $O(1)$.
    #[derive(Debug, Clone)]
    pub struct MinHeap<T: Ord> {
        inner: BinaryHeap<Reverse<T>>,
    }

    impl<T: Ord> Default for MinHeap<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Ord> MinHeap<T> {
        /// Cria uma min-heap vazia.
        #[inline]
        pub fn new() -> Self {
            MinHeap {
                inner: BinaryHeap::new(),
            }
        }

        /// Cria uma min-heap com capacidade pré-alocada.
        #[inline]
        pub fn with_capacity(capacity: usize) -> Self {
            MinHeap {
                inner: BinaryHeap::with_capacity(capacity),
            }
        }

        /// Constrói a min-heap em tempo linear $O(N)$ (`heapq.heapify`).
        pub fn from_vec(items: Vec<T>) -> Self {
            let revs: Vec<Reverse<T>> = items.into_iter().map(Reverse).collect();
            MinHeap {
                inner: BinaryHeap::from(revs),
            }
        }

        /// `heapq.heappush` — insere elemento em $O(\log N)$.
        #[inline]
        pub fn push(&mut self, valor: T) {
            self.inner.push(Reverse(valor));
        }

        /// `heapq.heappop` — remove e devolve o menor elemento em $O(\log N)$.
        #[inline]
        pub fn pop(&mut self) -> Option<T> {
            self.inner.pop().map(|Reverse(v)| v)
        }

        /// `heap[0]` — consulta o menor elemento em $O(1)$ sem remover.
        #[inline]
        pub fn peek(&self) -> Option<&T> {
            self.inner.peek().map(|Reverse(v)| v)
        }

        /// `heapq.heappushpop` — insere `valor` e extrai o menor resultante de forma eficiente.
        pub fn push_pop(&mut self, valor: T) -> T {
            if let Some(mut topo) = self.inner.peek_mut() {
                if valor > topo.0 {
                    let antigo = std::mem::replace(&mut topo.0, valor);
                    return antigo;
                }
            }
            valor
        }

        /// Quantidade de elementos na heap.
        #[inline]
        pub fn len(&self) -> usize {
            self.inner.len()
        }

        /// Retorna `true` se vazia.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }

        /// Remove todos os elementos.
        #[inline]
        pub fn clear(&mut self) {
            self.inner.clear();
        }

        /// Consome a min-heap devolvendo um `Vec<T>` em ordem crescente.
        pub fn into_sorted_vec(mut self) -> Vec<T> {
            let mut out = Vec::with_capacity(self.inner.len());
            while let Some(Reverse(v)) = self.inner.pop() {
                out.push(v);
            }
            out
        }
    }

    impl<T: Ord> FromIterator<T> for MinHeap<T> {
        fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
            MinHeap {
                inner: iter.into_iter().map(Reverse).collect(),
            }
        }
    }

    impl<T: Ord> PyLen for MinHeap<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    #[derive(Debug, Clone)]
    enum HeapMode<T: Ord> {
        Max(BinaryHeap<T>),
        Min(BinaryHeap<Reverse<T>>),
    }

    /// Fila de prioridade configurável como **min-heap** (`PriorityQueue::min_heap()`) ou
    /// **max-heap** (`PriorityQueue::max_heap()`).
    #[derive(Debug, Clone)]
    pub struct PriorityQueue<T: Ord> {
        mode: HeapMode<T>,
    }

    impl<T: Ord> PriorityQueue<T> {
        /// Cria uma fila de prioridade onde o **menor** item sai primeiro (`heapq`).
        #[inline]
        pub fn min_heap() -> Self {
            PriorityQueue {
                mode: HeapMode::Min(BinaryHeap::new()),
            }
        }

        /// Cria uma fila de prioridade onde o **maior** item sai primeiro (`std::collections::BinaryHeap`).
        #[inline]
        pub fn max_heap() -> Self {
            PriorityQueue {
                mode: HeapMode::Max(BinaryHeap::new()),
            }
        }

        /// Insere item na fila em $O(\log N)$.
        pub fn push(&mut self, valor: T) {
            match &mut self.mode {
                HeapMode::Max(h) => h.push(valor),
                HeapMode::Min(h) => h.push(Reverse(valor)),
            }
        }

        /// Remove e devolve o elemento de maior prioridade em $O(\log N)$.
        pub fn pop(&mut self) -> Option<T> {
            match &mut self.mode {
                HeapMode::Max(h) => h.pop(),
                HeapMode::Min(h) => h.pop().map(|Reverse(v)| v),
            }
        }

        /// Inspeciona o elemento de maior prioridade em $O(1)$.
        pub fn peek(&self) -> Option<&T> {
            match &self.mode {
                HeapMode::Max(h) => h.peek(),
                HeapMode::Min(h) => h.peek().map(|Reverse(v)| v),
            }
        }

        /// Quantidade de elementos na fila.
        pub fn len(&self) -> usize {
            match &self.mode {
                HeapMode::Max(h) => h.len(),
                HeapMode::Min(h) => h.len(),
            }
        }

        /// Retorna `true` se a fila estiver vazia.
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        /// Remove todos os elementos mantendo a política min/max.
        pub fn clear(&mut self) {
            match &mut self.mode {
                HeapMode::Max(h) => h.clear(),
                HeapMode::Min(h) => h.clear(),
            }
        }

        /// Consome a fila extraindo todos os elementos em ordem de prioridade.
        pub fn into_sorted_vec(self) -> Vec<T> {
            match self.mode {
                HeapMode::Max(mut h) => {
                    let mut out = Vec::with_capacity(h.len());
                    while let Some(v) = h.pop() {
                        out.push(v);
                    }
                    out
                }
                HeapMode::Min(mut h) => {
                    let mut out = Vec::with_capacity(h.len());
                    while let Some(Reverse(v)) = h.pop() {
                        out.push(v);
                    }
                    out
                }
            }
        }
    }

    impl<T: Ord> PyLen for PriorityQueue<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }
}

// ============================================================================
// 2C) Iteradores Ergonomicos (100% lazy sempre que aplicável)
// ============================================================================
pub mod iterator_utils {
    //! Funções livres e trait [`PyIterExt`] com combinadores preguiçosos sobre `IntoIterator` e
    //! `Iterator`, evitando alocações intermediárias e mantendo a inferência de tipos do Rust.

    use std::collections::HashMap;
    use std::hash::Hash;

    /// Representa um par de [`ZipLongest`] quando os dois iteradores têm comprimentos distintos.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EitherOrBoth<A, B> {
        Both(A, B),
        Left(A),
        Right(B),
    }

    /// Iterador lazy equivalente a `itertools.zip_longest(a, b)` do Python.
    pub struct ZipLongest<A: Iterator, B: Iterator> {
        a: A,
        b: B,
        a_esgotado: bool,
        b_esgotado: bool,
    }

    impl<A: Iterator, B: Iterator> Iterator for ZipLongest<A, B> {
        type Item = EitherOrBoth<A::Item, B::Item>;

        fn next(&mut self) -> Option<Self::Item> {
            let item_a = if self.a_esgotado {
                None
            } else {
                match self.a.next() {
                    Some(v) => Some(v),
                    None => {
                        self.a_esgotado = true;
                        None
                    }
                }
            };
            let item_b = if self.b_esgotado {
                None
            } else {
                match self.b.next() {
                    Some(v) => Some(v),
                    None => {
                        self.b_esgotado = true;
                        None
                    }
                }
            };
            match (item_a, item_b) {
                (Some(x), Some(y)) => Some(EitherOrBoth::Both(x, y)),
                (Some(x), None) => Some(EitherOrBoth::Left(x)),
                (None, Some(y)) => Some(EitherOrBoth::Right(y)),
                (None, None) => None,
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let (la, ua) = if self.a_esgotado {
                (0, Some(0))
            } else {
                self.a.size_hint()
            };
            let (lb, ub) = if self.b_esgotado {
                (0, Some(0))
            } else {
                self.b.size_hint()
            };
            let lower = la.max(lb);
            let upper = match (ua, ub) {
                (Some(x), Some(y)) => Some(x.max(y)),
                _ => None,
            };
            (lower, upper)
        }
    }

    /// Iterador lazy equivalente a `itertools.batched(iter, n)` (Python 3.12+).
    pub struct Batched<I: Iterator> {
        iter: I,
        tamanho: usize,
    }

    impl<I: Iterator> Iterator for Batched<I> {
        type Item = Vec<I::Item>;

        fn next(&mut self) -> Option<Self::Item> {
            let primeiro = self.iter.next()?;
            let mut lote = Vec::with_capacity(self.tamanho);
            lote.push(primeiro);
            for _ in 1..self.tamanho {
                match self.iter.next() {
                    Some(item) => lote.push(item),
                    None => break,
                }
            }
            Some(lote)
        }
    }

    /// Iterador lazy equivalente a `itertools.islice(iter, start, stop, step)`.
    pub struct ISlice<I: Iterator> {
        iter: I,
        started: bool,
        start: usize,
        stop: Option<usize>,
        step: usize,
        pos: usize,
    }

    impl<I: Iterator> Iterator for ISlice<I> {
        type Item = I::Item;

        fn next(&mut self) -> Option<Self::Item> {
            if !self.started {
                self.started = true;
                while self.pos < self.start {
                    self.iter.next()?;
                    self.pos += 1;
                }
            }
            if let Some(s) = self.stop {
                if self.pos >= s {
                    return None;
                }
            }
            let item = self.iter.next()?;
            self.pos += 1;

            // Avança (step - 1) posições para a próxima chamada
            for _ in 1..self.step {
                if let Some(s) = self.stop {
                    if self.pos >= s {
                        break;
                    }
                }
                if self.iter.next().is_none() {
                    break;
                }
                self.pos += 1;
            }
            Some(item)
        }
    }

    /// Contador lazy equivalente a `itertools.count(start, step)` (para quando há overflow em `i64`).
    #[derive(Debug, Clone, Copy)]
    pub struct CountFrom {
        atual: Option<i64>,
        passo: i64,
    }

    impl Iterator for CountFrom {
        type Item = i64;

        #[inline]
        fn next(&mut self) -> Option<i64> {
            let v = self.atual?;
            self.atual = v.checked_add(self.passo);
            Some(v)
        }
    }

    /// `map(f, iter)` — transformação lazy (sem alocar `Vec`).
    #[inline]
    pub fn map<I, B, F>(iter: I, f: F) -> std::iter::Map<I::IntoIter, F>
    where
        I: IntoIterator,
        F: FnMut(I::Item) -> B,
    {
        iter.into_iter().map(f)
    }

    /// `filter(pred, iter)` — filtragem lazy.
    #[inline]
    pub fn filter<I, F>(iter: I, pred: F) -> std::iter::Filter<I::IntoIter, F>
    where
        I: IntoIterator,
        F: FnMut(&I::Item) -> bool,
    {
        iter.into_iter().filter(pred)
    }

    /// Filtragem + transformação lazy em uma única passagem (`filter_map`).
    #[inline]
    pub fn filter_map<I, B, F>(iter: I, f: F) -> std::iter::FilterMap<I::IntoIter, F>
    where
        I: IntoIterator,
        F: FnMut(I::Item) -> Option<B>,
    {
        iter.into_iter().filter_map(f)
    }

    /// Transformação 1-para-muitos lazy (`flat_map`).
    #[inline]
    pub fn flat_map<I, U, F>(iter: I, f: F) -> std::iter::FlatMap<I::IntoIter, U, F>
    where
        I: IntoIterator,
        U: IntoIterator,
        F: FnMut(I::Item) -> U,
    {
        iter.into_iter().flat_map(f)
    }

    /// `itertools.chain(a, b)` — concatena dois iteráveis de forma lazy.
    #[inline]
    pub fn chain<A, B>(a: A, b: B) -> std::iter::Chain<A::IntoIter, B::IntoIter>
    where
        A: IntoIterator,
        B: IntoIterator<Item = A::Item>,
    {
        a.into_iter().chain(b)
    }

    /// Seleciona no máximo os primeiros `n` itens de forma lazy.
    #[inline]
    pub fn take<I: IntoIterator>(iter: I, n: usize) -> std::iter::Take<I::IntoIter> {
        iter.into_iter().take(n)
    }

    /// Descarta os primeiros `n` itens de forma lazy.
    #[inline]
    pub fn skip<I: IntoIterator>(iter: I, n: usize) -> std::iter::Skip<I::IntoIter> {
        iter.into_iter().skip(n)
    }

    /// `itertools.takewhile(pred, iter)` — lazy enquanto `pred` for `true`.
    #[inline]
    pub fn take_while<I, P>(iter: I, pred: P) -> std::iter::TakeWhile<I::IntoIter, P>
    where
        I: IntoIterator,
        P: FnMut(&I::Item) -> bool,
    {
        iter.into_iter().take_while(pred)
    }

    /// `itertools.dropwhile(pred, iter)` — descarta itens enquanto `pred` for `true`, depois emite tudo.
    #[inline]
    pub fn skip_while<I, P>(iter: I, pred: P) -> std::iter::SkipWhile<I::IntoIter, P>
    where
        I: IntoIterator,
        P: FnMut(&I::Item) -> bool,
    {
        iter.into_iter().skip_while(pred)
    }

    /// `itertools.chain.from_iterable(iter)` — achata um nível de aninhamento de forma lazy.
    #[inline]
    pub fn flatten<I>(iter: I) -> std::iter::Flatten<I::IntoIter>
    where
        I: IntoIterator,
        I::Item: IntoIterator,
    {
        iter.into_iter().flatten()
    }

    /// `enumerate(iter)` lazy.
    #[inline]
    pub fn enumerate<I: IntoIterator>(iter: I) -> std::iter::Enumerate<I::IntoIter> {
        iter.into_iter().enumerate()
    }

    /// Iterador lazy equivalente a `enumerate(iter, start)` sem alocação em heap.
    pub struct EnumerateFrom<I> {
        iter: I,
        pos: usize,
    }

    impl<I: Iterator> Iterator for EnumerateFrom<I> {
        type Item = (usize, I::Item);

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let item = self.iter.next()?;
            let idx = self.pos;
            self.pos += 1;
            Some((idx, item))
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }

    /// `enumerate(iter, start)` do Python — devolve pares `(start + i, item)` de forma lazy.
    #[inline]
    pub fn enumerate_from<I: IntoIterator>(
        iter: I,
        start: usize,
    ) -> EnumerateFrom<I::IntoIter> {
        EnumerateFrom {
            iter: iter.into_iter(),
            pos: start,
        }
    }

    /// `zip(a, b)` lazy (para no mais curto).
    #[inline]
    pub fn zip<A: IntoIterator, B: IntoIterator>(
        a: A,
        b: B,
    ) -> std::iter::Zip<A::IntoIter, B::IntoIter> {
        a.into_iter().zip(b)
    }

    /// `itertools.zip_longest(a, b)` lazy (continua até ambos esgotarem).
    #[inline]
    pub fn zip_longest<A: IntoIterator, B: IntoIterator>(
        a: A,
        b: B,
    ) -> ZipLongest<A::IntoIter, B::IntoIter> {
        ZipLongest {
            a: a.into_iter(),
            b: b.into_iter(),
            a_esgotado: false,
            b_esgotado: false,
        }
    }

    /// `itertools.islice(iter, start, stop, step)` lazy.
    ///
    /// # Panics
    /// Causa panic se `step == 0`.
    #[inline]
    pub fn islice<I: IntoIterator>(
        iter: I,
        start: usize,
        stop: Option<usize>,
        step: usize,
    ) -> ISlice<I::IntoIter> {
        assert!(step > 0, "ValueError: step em islice() deve ser >= 1");
        ISlice {
            iter: iter.into_iter(),
            started: false,
            start,
            stop,
            step,
            pos: 0,
        }
    }

    /// `itertools.batched(iter, size)` lazy.
    ///
    /// # Panics
    /// Causa panic se `size == 0`.
    #[inline]
    pub fn batched<I: IntoIterator>(iter: I, size: usize) -> Batched<I::IntoIter> {
        assert!(size > 0, "ValueError: batched() requer size >= 1");
        Batched {
            iter: iter.into_iter(),
            tamanho: size,
        }
    }

    /// `itertools.count(start, step)` lazy (para limiar de `i64` ao atingir overflow).
    #[inline]
    pub fn count_from(start: i64, step: i64) -> CountFrom {
        CountFrom {
            atual: Some(start),
            passo: step,
        }
    }

    /// Separa os itens em dois vetores `(verdadeiros, falsos)` segundo o predicado `pred`.
    #[inline]
    pub fn partition<I, F>(iter: I, pred: F) -> (Vec<I::Item>, Vec<I::Item>)
    where
        I: IntoIterator,
        F: FnMut(&I::Item) -> bool,
    {
        iter.into_iter().partition(pred)
    }

    /// `functools.reduce(f, iter, initial)` (`fold`).
    #[inline]
    pub fn fold<I, B, F>(iter: I, init: B, f: F) -> B
    where
        I: IntoIterator,
        F: FnMut(B, I::Item) -> B,
    {
        iter.into_iter().fold(init, f)
    }

    /// `functools.reduce(f, iter)` — retorna `None` se o iterável estiver vazio.
    #[inline]
    pub fn reduce<I, F>(iter: I, f: F) -> Option<I::Item>
    where
        I: IntoIterator,
        F: FnMut(I::Item, I::Item) -> I::Item,
    {
        iter.into_iter().reduce(f)
    }

    /// `any(pred(x) for x in iter)` — curto-circuita ao encontrar o primeiro `true`.
    #[inline]
    pub fn any_match<I, F>(iter: I, pred: F) -> bool
    where
        I: IntoIterator,
        F: FnMut(I::Item) -> bool,
    {
        iter.into_iter().any(pred)
    }

    /// `all(pred(x) for x in iter)` — curto-circuita ao encontrar o primeiro `false`.
    #[inline]
    pub fn all_match<I, F>(iter: I, pred: F) -> bool
    where
        I: IntoIterator,
        F: FnMut(I::Item) -> bool,
    {
        iter.into_iter().all(pred)
    }

    /// Extension trait sobre qualquer `Iterator` com utilitários que não colidem com `std::iter::Iterator`.
    pub trait PyIterExt: Iterator + Sized {
        /// Coleta o iterador em `Vec<Self::Item>` sem precisar especificar turbofish.
        #[inline]
        fn collect_vec(self) -> Vec<Self::Item> {
            self.collect()
        }

        /// Enumera começando do índice `start`.
        #[inline]
        fn enumerate_from(self, start: usize) -> EnumerateFrom<Self> {
            enumerate_from(self, start)
        }

        /// Combina este iterador com `other` até que ambos terminem.
        #[inline]
        fn zip_longest<B: IntoIterator>(self, other: B) -> ZipLongest<Self, B::IntoIter> {
            zip_longest(self, other)
        }

        /// Agrupa elementos em lotes de tamanho `size`.
        #[inline]
        fn batched(self, size: usize) -> Batched<Self> {
            batched(self, size)
        }

        /// Fatiamento preguiçoso `(start, stop, step)`.
        #[inline]
        fn islice(self, start: usize, stop: Option<usize>, step: usize) -> ISlice<Self> {
            islice(self, start, stop, step)
        }

        /// Conta frequências de cada elemento em um `HashMap<Item, usize>`.
        fn frequencies(self) -> HashMap<Self::Item, usize>
        where
            Self::Item: Eq + Hash,
        {
            let (lo, _) = self.size_hint();
            let mut map = HashMap::with_capacity(lo.min(1024));
            for item in self {
                *map.entry(item).or_insert(0) += 1;
            }
            map
        }

        /// Retorna `true` se ao menos um `bool` for `true` (para `Iterator<Item = bool>`).
        #[inline]
        fn any_true(self) -> bool
        where
            Self: Iterator<Item = bool>,
        {
            self.into_iter().any(|x| x)
        }

        /// Retorna `true` se todos os `bool` forem `true` (para `Iterator<Item = bool>`).
        #[inline]
        fn all_true(self) -> bool
        where
            Self: Iterator<Item = bool>,
        {
            self.into_iter().all(|x| x)
        }

        /// Conta quantos elementos satisfazem `pred`.
        #[inline]
        fn count_where<P: FnMut(&Self::Item) -> bool>(self, pred: P) -> usize {
            self.filter(pred).count()
        }

        /// Particiona em dois `Vec`s conforme `pred`.
        #[inline]
        fn partition_vec<P: FnMut(&Self::Item) -> bool>(
            self,
            pred: P,
        ) -> (Vec<Self::Item>, Vec<Self::Item>) {
            self.partition(pred)
        }
    }

    impl<I: Iterator> PyIterExt for I {}
}

// ============================================================================
// 3) Matriz — dados contíguos (cache-friendly), bounds mínimos e try_new
// ============================================================================
pub mod matrix_utils {
    //! Matriz bidimensional densa com armazenamento contíguo (`Vec<T>` linear).
    //!
    //! Garante localidade espacial na memória e permite iteração sequencial por linhas
    //! sem alocações por linha (ao contrário de `Vec<Vec<T>>`).

    use super::arithmetic::NumeroPy;
    use super::builtins::PyLen;
    use std::fmt;

    /// Erros retornados por operações falíveis sobre [`Matrix`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MatrixError {
        /// Dimensões ou tamanho do vetor de entrada inconsistentes.
        DimensoesInvalidas(String),
        /// O produto `linhas * colunas` excedeu `usize::MAX`.
        OverflowDeDimensao,
        /// A alocação do buffer contíguo falhou (ex.: falta de memória).
        FalhaDeAlocacao,
        /// As dimensões das matrizes não são compatíveis para a operação solicitada.
        DimensoesIncompativeis {
            esperado: (usize, usize),
            recebido: (usize, usize),
        },
    }

    impl fmt::Display for MatrixError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MatrixError::DimensoesInvalidas(msg) => write!(f, "ValueError: {}", msg),
                MatrixError::OverflowDeDimensao => {
                    write!(f, "OverflowError: linhas * colunas excede usize::MAX")
                }
                MatrixError::FalhaDeAlocacao => {
                    write!(f, "MemoryError: falha ao alocar buffer da matriz")
                }
                MatrixError::DimensoesIncompativeis { esperado, recebido } => write!(
                    f,
                    "ValueError: dimensões incompatíveis (esperado {:?}, recebido {:?})",
                    esperado, recebido
                ),
            }
        }
    }

    impl std::error::Error for MatrixError {}

    /// Matriz bidimensional densa armazenada em row-major order.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Matrix<T> {
        dados: Vec<T>,
        pub linhas: usize,
        pub colunas: usize,
    }

    /// Iterador sobre as fatias (`&[T]`) de cada linha da [`Matrix`].
    /// Trata corretamente o caso `colunas == 0` (ao contrário de `chunks_exact(0)`).
    pub struct MatrixRows<'a, T> {
        dados: &'a [T],
        colunas: usize,
        atual: usize,
        linhas: usize,
    }

    impl<'a, T> Iterator for MatrixRows<'a, T> {
        type Item = &'a [T];

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.atual >= self.linhas {
                return None;
            }
            let ini = self.atual * self.colunas;
            self.atual += 1;
            Some(&self.dados[ini..ini + self.colunas])
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let rem = self.linhas.saturating_sub(self.atual);
            (rem, Some(rem))
        }
    }

    impl<'a, T> DoubleEndedIterator for MatrixRows<'a, T> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.atual >= self.linhas {
                return None;
            }
            self.linhas -= 1;
            let ini = self.linhas * self.colunas;
            Some(&self.dados[ini..ini + self.colunas])
        }
    }

    impl<'a, T> ExactSizeIterator for MatrixRows<'a, T> {}

    /// Iterador lazy sobre os elementos de uma coluna específica da [`Matrix`].
    pub struct MatrixCol<'a, T> {
        dados: &'a [T],
        colunas: usize,
        coluna: usize,
        atual: usize,
        linhas: usize,
    }

    impl<'a, T> Iterator for MatrixCol<'a, T> {
        type Item = &'a T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.atual >= self.linhas {
                return None;
            }
            let idx = self.atual * self.colunas + self.coluna;
            self.atual += 1;
            Some(&self.dados[idx])
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let rem = self.linhas.saturating_sub(self.atual);
            (rem, Some(rem))
        }
    }

    impl<'a, T> ExactSizeIterator for MatrixCol<'a, T> {}

    impl<T> Matrix<T> {
        /// Constrói uma matriz a partir de um `Vec<T>` linear pré-alocado, validando
        /// `linhas * colunas == dados.len()` sem overflow.
        pub fn from_vec(
            linhas: usize,
            colunas: usize,
            dados: Vec<T>,
        ) -> Result<Self, MatrixError> {
            let total = linhas
                .checked_mul(colunas)
                .ok_or(MatrixError::OverflowDeDimensao)?;
            if total != dados.len() {
                return Err(MatrixError::DimensoesInvalidas(format!(
                    "{}x{} exige {} elementos, mas o vetor possui {}",
                    linhas,
                    colunas,
                    total,
                    dados.len()
                )));
            }
            Ok(Matrix {
                dados,
                linhas,
                colunas,
            })
        }

        /// Devolve `(linhas, colunas)`.
        #[inline]
        pub const fn shape(&self) -> (usize, usize) {
            (self.linhas, self.colunas)
        }

        /// Número total de células (`linhas * colunas`).
        #[inline]
        pub fn len(&self) -> usize {
            self.dados.len()
        }

        /// Retorna `true` se a matriz não possuir células (`linhas == 0 || colunas == 0`).
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.dados.is_empty()
        }

        /// Fatia linear contígua subjacente em ordem row-major.
        #[inline]
        pub fn as_slice(&self) -> &[T] {
            &self.dados
        }

        /// Fatia linear mutável subjacente em ordem row-major.
        #[inline]
        pub fn as_mut_slice(&mut self) -> &mut [T] {
            &mut self.dados
        }

        /// Acesso seguro com verificação de limites (`Option<&T>`).
        #[inline]
        pub fn get(&self, linha: usize, coluna: usize) -> Option<&T> {
            if linha < self.linhas && coluna < self.colunas {
                Some(&self.dados[linha * self.colunas + coluna])
            } else {
                None
            }
        }

        /// Acesso mutável seguro com verificação de limites (`Option<&mut T>`).
        #[inline]
        pub fn get_mut(&mut self, linha: usize, coluna: usize) -> Option<&mut T> {
            if linha < self.linhas && coluna < self.colunas {
                Some(&mut self.dados[linha * self.colunas + coluna])
            } else {
                None
            }
        }

        /// Devolve uma fatia (`&[T]`) da linha `linha` em $O(1)$, sem alocar.
        #[inline]
        pub fn row(&self, linha: usize) -> Option<&[T]> {
            if linha < self.linhas {
                let ini = linha * self.colunas;
                Some(&self.dados[ini..ini + self.colunas])
            } else {
                None
            }
        }

        /// Devolve uma fatia mutável (`&mut [T]`) da linha `linha` em $O(1)$, sem alocar.
        #[inline]
        pub fn row_mut(&mut self, linha: usize) -> Option<&mut [T]> {
            if linha < self.linhas {
                let ini = linha * self.colunas;
                Some(&mut self.dados[ini..ini + self.colunas])
            } else {
                None
            }
        }

        /// Iterador lazy sobre todas as linhas como fatias `&[T]`.
        #[inline]
        pub fn rows(&self) -> MatrixRows<'_, T> {
            MatrixRows {
                dados: &self.dados,
                colunas: self.colunas,
                atual: 0,
                linhas: self.linhas,
            }
        }

        /// Iterador lazy sobre todos os elementos da coluna `coluna`.
        #[inline]
        pub fn col(&self, coluna: usize) -> Option<MatrixCol<'_, T>> {
            if coluna >= self.colunas {
                None
            } else {
                Some(MatrixCol {
                    dados: &self.dados,
                    colunas: self.colunas,
                    coluna,
                    atual: 0,
                    linhas: self.linhas,
                })
            }
        }

        /// Iterador linear sobre todos os elementos (row-major).
        #[inline]
        pub fn iter(&self) -> std::slice::Iter<'_, T> {
            self.dados.iter()
        }

        /// Iterador mutável linear sobre todos os elementos (row-major).
        #[inline]
        pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
            self.dados.iter_mut()
        }

        /// Consome a matriz e devolve o `Vec<T>` linear subjacente sem cópia.
        #[inline]
        pub fn flatten(self) -> Vec<T> {
            self.dados
        }

        /// Aplica `f` a cada elemento gerando uma nova `Matrix<U>` de mesmas dimensões.
        pub fn map<U, F: FnMut(&T) -> U>(&self, f: F) -> Matrix<U> {
            Matrix {
                dados: self.dados.iter().map(f).collect(),
                linhas: self.linhas,
                colunas: self.colunas,
            }
        }
    }

    impl<T: Clone> Matrix<T> {
        /// Cria uma matriz `linhas x colunas` preenchida com `valor_padrao`.
        ///
        /// # Panics
        /// Causa panic se `linhas * colunas` exceder `usize::MAX`. Para construção falível
        /// controlada, use [`Matrix::try_new`].
        pub fn new(linhas: usize, colunas: usize, valor_padrao: T) -> Self {
            let total = linhas
                .checked_mul(colunas)
                .expect("OverflowError: linhas * colunas excede usize::MAX em Matrix::new");
            Matrix {
                dados: vec![valor_padrao; total],
                linhas,
                colunas,
            }
        }

        /// Versão segura de [`Matrix::new`]: valida overflow de dimensões e usa
        /// `try_reserve_exact` antes de preencher os dados.
        pub fn try_new(
            linhas: usize,
            colunas: usize,
            valor_padrao: T,
        ) -> Result<Self, MatrixError> {
            let total = linhas
                .checked_mul(colunas)
                .ok_or(MatrixError::OverflowDeDimensao)?;
            let mut dados = Vec::new();
            dados
                .try_reserve_exact(total)
                .map_err(|_| MatrixError::FalhaDeAlocacao)?;
            dados.resize(total, valor_padrao);
            Ok(Matrix {
                dados,
                linhas,
                colunas,
            })
        }

        /// Cria uma matriz a partir de uma função geradora `f(linha, coluna)`.
        ///
        /// # Panics
        /// Causa panic se `linhas * colunas` exceder `usize::MAX`.
        pub fn from_fn<F>(linhas: usize, colunas: usize, mut f: F) -> Self
        where
            F: FnMut(usize, usize) -> T,
        {
            let total = linhas
                .checked_mul(colunas)
                .expect("OverflowError: linhas * colunas excede usize::MAX em Matrix::from_fn");
            let mut dados = Vec::with_capacity(total);
            for i in 0..linhas {
                for j in 0..colunas {
                    dados.push(f(i, j));
                }
            }
            Matrix {
                dados,
                linhas,
                colunas,
            }
        }

        /// Constrói uma matriz a partir de uma lista de linhas verificando retangularidade.
        pub fn from_rows<R: AsRef<[T]>>(rows: &[R]) -> Result<Self, MatrixError> {
            let linhas = rows.len();
            if linhas == 0 {
                return Ok(Matrix {
                    dados: Vec::new(),
                    linhas: 0,
                    colunas: 0,
                });
            }
            let colunas = rows[0].as_ref().len();
            let total = linhas
                .checked_mul(colunas)
                .ok_or(MatrixError::OverflowDeDimensao)?;
            let mut dados = Vec::new();
            dados
                .try_reserve_exact(total)
                .map_err(|_| MatrixError::FalhaDeAlocacao)?;
            for (idx, r) in rows.iter().enumerate() {
                let fatia = r.as_ref();
                if fatia.len() != colunas {
                    return Err(MatrixError::DimensoesInvalidas(format!(
                        "linha {} possui {} colunas, esperado {}",
                        idx,
                        fatia.len(),
                        colunas
                    )));
                }
                dados.extend_from_slice(fatia);
            }
            Ok(Matrix {
                dados,
                linhas,
                colunas,
            })
        }

        /// Devolve a matriz transposta ($M^T$) em passagem única ($1\times$ clone por célula,
        /// sem inicialização redundante).
        pub fn transpose(&self) -> Matrix<T> {
            if self.dados.is_empty() {
                return Matrix {
                    dados: Vec::new(),
                    linhas: self.colunas,
                    colunas: self.linhas,
                };
            }
            let mut dados = Vec::with_capacity(self.dados.len());
            for j in 0..self.colunas {
                for i in 0..self.linhas {
                    dados.push(self.dados[i * self.colunas + j].clone());
                }
            }
            Matrix {
                dados,
                linhas: self.colunas,
                colunas: self.linhas,
            }
        }
    }

    impl<T: NumeroPy> Matrix<T> {
        /// Cria uma matriz `linhas x colunas` preenchida com zeros (`T::ZERO`).
        pub fn zeros(linhas: usize, colunas: usize) -> Self {
            Matrix::new(linhas, colunas, T::ZERO)
        }

        /// Cria a matriz identidade $I_n$ (`n x n`).
        pub fn identity(n: usize) -> Self {
            let mut m = Matrix::zeros(n, n);
            for i in 0..n {
                m.dados[i * n + i] = T::UM;
            }
            m
        }

        /// Multiplicação matricial (`self * other`) com transposição interna para leitura
        /// sequencial na cache. Devolve `None` se `self.colunas != other.linhas`.
        pub fn dot(&self, other: &Matrix<T>) -> Option<Matrix<T>> {
            self.try_dot(other).ok()
        }

        /// Versão com erro detalhado da multiplicação matricial (`self * other`).
        pub fn try_dot(&self, other: &Matrix<T>) -> Result<Matrix<T>, MatrixError> {
            if self.colunas != other.linhas {
                return Err(MatrixError::DimensoesIncompativeis {
                    esperado: (self.colunas, other.colunas),
                    recebido: (other.linhas, other.colunas),
                });
            }
            let n = self.colunas;
            let other_t = other.transpose();
            let mut res = Matrix::try_new(self.linhas, other.colunas, T::ZERO)?;

            for i in 0..self.linhas {
                let linha_a = &self.dados[i * n..(i + 1) * n];
                for j in 0..other.colunas {
                    let linha_bt = &other_t.dados[j * n..(j + 1) * n];
                    let mut soma = T::ZERO;
                    for k in 0..n {
                        soma = soma + linha_a[k] * linha_bt[k];
                    }
                    res.dados[i * other.colunas + j] = soma;
                }
            }
            Ok(res)
        }

        /// Soma elemento a elemento (`self + other`).
        pub fn add(&self, other: &Matrix<T>) -> Result<Matrix<T>, MatrixError> {
            if self.shape() != other.shape() {
                return Err(MatrixError::DimensoesIncompativeis {
                    esperado: self.shape(),
                    recebido: other.shape(),
                });
            }
            let dados = self
                .dados
                .iter()
                .zip(other.dados.iter())
                .map(|(&a, &b)| a + b)
                .collect();
            Ok(Matrix {
                dados,
                linhas: self.linhas,
                colunas: self.colunas,
            })
        }

        /// Subtração elemento a elemento (`self - other`).
        pub fn sub(&self, other: &Matrix<T>) -> Result<Matrix<T>, MatrixError> {
            if self.shape() != other.shape() {
                return Err(MatrixError::DimensoesIncompativeis {
                    esperado: self.shape(),
                    recebido: other.shape(),
                });
            }
            let dados = self
                .dados
                .iter()
                .zip(other.dados.iter())
                .map(|(&a, &b)| a - b)
                .collect();
            Ok(Matrix {
                dados,
                linhas: self.linhas,
                colunas: self.colunas,
            })
        }

        /// Produto de Hadamard (multiplicação elemento a elemento).
        pub fn hadamard(&self, other: &Matrix<T>) -> Result<Matrix<T>, MatrixError> {
            if self.shape() != other.shape() {
                return Err(MatrixError::DimensoesIncompativeis {
                    esperado: self.shape(),
                    recebido: other.shape(),
                });
            }
            let dados = self
                .dados
                .iter()
                .zip(other.dados.iter())
                .map(|(&a, &b)| a * b)
                .collect();
            Ok(Matrix {
                dados,
                linhas: self.linhas,
                colunas: self.colunas,
            })
        }

        /// Multiplica todos os elementos por um escalar.
        pub fn scale(&self, fator: T) -> Matrix<T> {
            Matrix {
                dados: self.dados.iter().map(|&x| x * fator).collect(),
                linhas: self.linhas,
                colunas: self.colunas,
            }
        }

        /// Traço da matriz (soma da diagonal principal); requer matriz quadrada.
        pub fn trace(&self) -> Result<T, MatrixError> {
            if self.linhas != self.colunas {
                return Err(MatrixError::DimensoesIncompativeis {
                    esperado: (self.linhas, self.linhas),
                    recebido: self.shape(),
                });
            }
            let mut soma = T::ZERO;
            for i in 0..self.linhas {
                soma = soma + self.dados[i * self.colunas + i];
            }
            Ok(soma)
        }
    }

    impl<T> PyLen for Matrix<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.linhas
        }
    }

    impl<T> std::ops::Index<(usize, usize)> for Matrix<T> {
        type Output = T;
        #[inline]
        fn index(&self, (linha, coluna): (usize, usize)) -> &T {
            assert!(
                linha < self.linhas && coluna < self.colunas,
                "IndexError: índice ({}, {}) fora da matriz {}x{}",
                linha,
                coluna,
                self.linhas,
                self.colunas
            );
            &self.dados[linha * self.colunas + coluna]
        }
    }

    impl<T> std::ops::IndexMut<(usize, usize)> for Matrix<T> {
        #[inline]
        fn index_mut(&mut self, (linha, coluna): (usize, usize)) -> &mut T {
            assert!(
                linha < self.linhas && coluna < self.colunas,
                "IndexError: índice ({}, {}) fora da matriz {}x{}",
                linha,
                coluna,
                self.linhas,
                self.colunas
            );
            &mut self.dados[linha * self.colunas + coluna]
        }
    }
}

// ============================================================================
// 4) Built-ins gerais
// ============================================================================
pub mod builtins {
    //! Funções livres equivalentes às funções integradas (*built-ins*) do Python.
    //!
    //! Na v2.0.0:
    //! - Distingue explicitamente [`min`]/[`max`]/[`sorted`] (`PartialOrd`) de [`min_ord`]/
    //!   [`max_ord`]/[`sorted_ord`] (`Ord`), além de [`try_min`]/[`try_max`]/[`try_sorted`]
    //!   sem panic e [`sorted_f64_total`] via `f64::total_cmp`.
    //! - Corrige a dependência de ordem de `NaN` em [`min`] e [`max`].

    pub use super::arithmetic::{abs, sum};

    use std::borrow::Cow;
    use std::cell::Cell;
    use std::cmp::Ordering;
    use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
    use std::fmt::{self, Display};
    use std::hash::{BuildHasher, Hash};

    /// Erro quando uma comparação falha (ex.: valores `NaN` em `PartialOrd`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CompareError {
        Incomparable,
    }

    impl fmt::Display for CompareError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                CompareError::Incomparable => {
                    write!(f, "ValueError: valores incomparáveis encontrados (ex.: NaN)")
                }
            }
        }
    }

    impl std::error::Error for CompareError {}

    /// `print(x)` — exibe qualquer valor que implemente `Display` seguido de quebra de linha.
    #[inline]
    pub fn print<T: Display>(valor: T) {
        println!("{}", valor);
    }

    /// Imprime um valor `Display` na saída de erro padrão (`stderr`) seguido de quebra de linha.
    #[inline]
    pub fn eprint<T: Display>(valor: T) {
        eprintln!("{}", valor);
    }

    /// `len(&x)` — devolve o comprimento da coleção ou string.
    ///
    /// Nota: para `str` e `String`, retorna a quantidade de caracteres Unicode escalares
    /// (`chars().count()`), equivalente ao `len()` do Python e diferente de `.len()` (bytes UTF-8)
    /// do Rust.
    #[inline]
    pub fn len<T: ?Sized + PyLen>(x: &T) -> usize {
        x.py_len()
    }

    /// Trait de extensão para o built-in [`len`].
    pub trait PyLen {
        fn py_len(&self) -> usize;
    }

    impl PyLen for str {
        #[inline]
        fn py_len(&self) -> usize {
            self.chars().count()
        }
    }

    impl PyLen for String {
        #[inline]
        fn py_len(&self) -> usize {
            self.chars().count()
        }
    }

    impl PyLen for Box<str> {
        #[inline]
        fn py_len(&self) -> usize {
            self.chars().count()
        }
    }

    impl<'a> PyLen for Cow<'a, str> {
        #[inline]
        fn py_len(&self) -> usize {
            self.chars().count()
        }
    }

    impl<T> PyLen for [T] {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<T, const N: usize> PyLen for [T; N] {
        #[inline]
        fn py_len(&self) -> usize {
            N
        }
    }

    impl<T> PyLen for Vec<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<T> PyLen for Box<[T]> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<'a, T: Clone> PyLen for Cow<'a, [T]> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<T> PyLen for VecDeque<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<T> PyLen for BinaryHeap<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<K, V, S> PyLen for HashMap<K, V, S> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<T, S> PyLen for HashSet<T, S> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<K, V> PyLen for BTreeMap<K, V> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<T> PyLen for BTreeSet<T> {
        #[inline]
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    /// `min(iter)` para `PartialOrd` — ignora valores incomparáveis consigo mesmos (como `NaN`)
    /// de forma independente da posição no iterador; devolve `None` se vazio ou se todos os
    /// elementos forem incomparáveis.
    ///
    /// Para erro explícito ao encontrar `NaN`, use [`try_min`]. Para tipos `Ord`, use [`min_ord`].
    pub fn min<T, I>(iter: I) -> Option<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut menor: Option<T> = None;
        for x in iter {
            if x.partial_cmp(&x).is_none() {
                continue;
            }
            menor = match menor {
                None => Some(x),
                Some(m) => {
                    if x < m {
                        Some(x)
                    } else {
                        Some(m)
                    }
                }
            };
        }
        menor
    }

    /// `max(iter)` para `PartialOrd` — ignora valores incomparáveis consigo mesmos (`NaN`) de
    /// forma consistente independente da posição. Para falhar em `NaN`, use [`try_max`].
    pub fn max<T, I>(iter: I) -> Option<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut maior: Option<T> = None;
        for x in iter {
            if x.partial_cmp(&x).is_none() {
                continue;
            }
            maior = match maior {
                None => Some(x),
                Some(m) => {
                    if x > m {
                        Some(x)
                    } else {
                        Some(m)
                    }
                }
            };
        }
        maior
    }

    /// Versão estrita de [`min`] que retorna `Err(CompareError::Incomparable)` caso qualquer
    /// elemento não seja comparável (ex.: `f64::NAN`).
    pub fn try_min<T, I>(iter: I) -> Result<Option<T>, CompareError>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut menor: Option<T> = None;
        for x in iter {
            if x.partial_cmp(&x).is_none() {
                return Err(CompareError::Incomparable);
            }
            menor = match menor {
                None => Some(x),
                Some(m) => match x.partial_cmp(&m) {
                    Some(Ordering::Less) => Some(x),
                    Some(_) => Some(m),
                    None => return Err(CompareError::Incomparable),
                },
            };
        }
        Ok(menor)
    }

    /// Versão estrita de [`max`] que retorna `Err(CompareError::Incomparable)` em caso de `NaN`.
    pub fn try_max<T, I>(iter: I) -> Result<Option<T>, CompareError>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut maior: Option<T> = None;
        for x in iter {
            if x.partial_cmp(&x).is_none() {
                return Err(CompareError::Incomparable);
            }
            maior = match maior {
                None => Some(x),
                Some(m) => match x.partial_cmp(&m) {
                    Some(Ordering::Greater) => Some(x),
                    Some(_) => Some(m),
                    None => return Err(CompareError::Incomparable),
                },
            };
        }
        Ok(maior)
    }

    /// `min(iter)` com ordem total garantida (`T: Ord`).
    #[inline]
    pub fn min_ord<T: Ord, I: IntoIterator<Item = T>>(iter: I) -> Option<T> {
        iter.into_iter().min()
    }

    /// `max(iter)` com ordem total garantida (`T: Ord`).
    #[inline]
    pub fn max_ord<T: Ord, I: IntoIterator<Item = T>>(iter: I) -> Option<T> {
        iter.into_iter().max()
    }

    /// `min(iter, key=f)` — devolve o elemento cujo valor de chave `f(&x)` é mínimo.
    #[inline]
    pub fn min_by_key<T, K: Ord, I, F>(iter: I, f: F) -> Option<T>
    where
        I: IntoIterator<Item = T>,
        F: FnMut(&T) -> K,
    {
        iter.into_iter().min_by_key(f)
    }

    /// `max(iter, key=f)` — devolve o elemento cujo valor de chave `f(&x)` é máximo.
    #[inline]
    pub fn max_by_key<T, K: Ord, I, F>(iter: I, f: F) -> Option<T>
    where
        I: IntoIterator<Item = T>,
        F: FnMut(&T) -> K,
    {
        iter.into_iter().max_by_key(f)
    }

    /// `sorted(iter)` — devolve um novo `Vec<T>` ordenado em ordem crescente (`PartialOrd`).
    ///
    /// # Panics
    /// Causa panic se encontrar pares incomparáveis (como `NaN`). Para versão sem panic, use
    /// [`try_sorted`], [`sorted_ord`] ou [`sorted_f64_total`].
    pub fn sorted<T, I>(iter: I) -> Vec<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut v: Vec<T> = iter.into_iter().collect();
        v.sort_by(|a, b| {
            a.partial_cmp(b)
                .expect("ValueError: valores incomparáveis em sorted() (ex.: NaN)")
        });
        v
    }

    /// Versão segura de [`sorted`] que retorna `Err(CompareError::Incomparable)` se encontrar
    /// valores como `NaN`, sem causar panic.
    pub fn try_sorted<T, I>(iter: I) -> Result<Vec<T>, CompareError>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut v: Vec<T> = iter.into_iter().collect();
        for item in &v {
            if item.partial_cmp(item).is_none() {
                return Err(CompareError::Incomparable);
            }
        }
        let falhou = Cell::new(false);
        v.sort_by(|a, b| match a.partial_cmp(b) {
            Some(ord) => ord,
            None => {
                falhou.set(true);
                Ordering::Equal
            }
        });
        if falhou.get() {
            Err(CompareError::Incomparable)
        } else {
            Ok(v)
        }
    }

    /// `sorted(iter)` para tipos com ordem total (`T: Ord`), sem risco de panic.
    #[inline]
    pub fn sorted_ord<T: Ord, I: IntoIterator<Item = T>>(iter: I) -> Vec<T> {
        let mut v: Vec<T> = iter.into_iter().collect();
        v.sort();
        v
    }

    /// `sorted(iter, reverse=True)` — ordena em ordem decrescente (`PartialOrd`).
    ///
    /// # Panics
    /// Causa panic em valores incomparáveis (`NaN`).
    pub fn sorted_desc<T, I>(iter: I) -> Vec<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut v: Vec<T> = iter.into_iter().collect();
        v.sort_by(|a, b| {
            b.partial_cmp(a)
                .expect("ValueError: valores incomparáveis em sorted_desc() (ex.: NaN)")
        });
        v
    }

    /// `sorted(iter, key=f)` — devolve um novo `Vec<T>` ordenado pela chave `K: Ord`.
    #[inline]
    pub fn sorted_by_key<T, K: Ord, I, F>(iter: I, f: F) -> Vec<T>
    where
        I: IntoIterator<Item = T>,
        F: FnMut(&T) -> K,
    {
        let mut v: Vec<T> = iter.into_iter().collect();
        v.sort_by_key(f);
        v
    }

    /// Ordena `f64` utilizando ordem total IEEE-754 (`f64::total_cmp`), sem panic mesmo com `NaN`.
    #[inline]
    pub fn sorted_f64_total<I: IntoIterator<Item = f64>>(iter: I) -> Vec<f64> {
        let mut v: Vec<f64> = iter.into_iter().collect();
        v.sort_by(f64::total_cmp);
        v
    }

    /// `reversed(iter)` — devolve um iterador reverso preguiçoso (sem alocar).
    #[inline]
    pub fn reversed<I>(iter: I) -> std::iter::Rev<I::IntoIter>
    where
        I: IntoIterator,
        I::IntoIter: DoubleEndedIterator,
    {
        iter.into_iter().rev()
    }

    /// `enumerate(iter)` — produz pares `(indice_usize, elemento)` igual ao Python.
    #[inline]
    pub fn enumerate<I: IntoIterator>(iter: I) -> std::iter::Enumerate<I::IntoIter> {
        iter.into_iter().enumerate()
    }

    /// `enumerate(iter, start)` — produz pares `(start + i, elemento)` de forma lazy.
    #[inline]
    pub fn enumerate_from<I: IntoIterator>(
        iter: I,
        start: usize,
    ) -> super::iterator_utils::EnumerateFrom<I::IntoIter> {
        super::iterator_utils::enumerate_from(iter, start)
    }

    /// `zip(a, b)` — caminha por duas sequências em paralelo produzindo tuplas `(a_i, b_i)`.
    #[inline]
    pub fn zip<A: IntoIterator, B: IntoIterator>(
        a: A,
        b: B,
    ) -> std::iter::Zip<A::IntoIter, B::IntoIter> {
        a.into_iter().zip(b)
    }

    /// `any(iter)` — devolve `true` se ao menos um elemento da sequência for `true`.
    #[inline]
    pub fn any<I: IntoIterator<Item = bool>>(iter: I) -> bool {
        iter.into_iter().any(|x| x)
    }

    /// `all(iter)` — devolve `true` se todos os elementos da sequência forem `true`.
    #[inline]
    pub fn all<I: IntoIterator<Item = bool>>(iter: I) -> bool {
        iter.into_iter().all(|x| x)
    }

    /// `histogram(iter)` — equivalente a `collections.Counter(iter)` do Python.
    /// Conta quantas vezes cada item aparece na sequência em tempo $O(N)$ médio.
    pub fn histogram<T, I>(iter: I) -> HashMap<T, usize>
    where
        I: IntoIterator<Item = T>,
        T: Eq + Hash,
    {
        let it = iter.into_iter();
        let (lo, _) = it.size_hint();
        let mut mapa = HashMap::with_capacity(lo.min(1024));
        for item in it {
            *mapa.entry(item).or_insert(0) += 1;
        }
        mapa
    }

    /// `type_of(&x)` — devolve o nome estático do tipo inferido pelo compilador.
    #[inline]
    pub fn type_of<T>(_: &T) -> &'static str {
        std::any::type_name::<T>()
    }

    /// Restringe `val` ao intervalo `[min_v, max_v]` para tipos com ordem total (`T: Ord`).
    ///
    /// # Panics
    /// Causa panic se `min_v > max_v` (mesma semântica de `Ord::clamp` da `std`).
    #[inline]
    pub fn clamp<T: Ord>(val: T, min_v: T, max_v: T) -> T {
        val.clamp(min_v, max_v)
    }

    /// Versão segura de `clamp` para tipos `PartialOrd` (incluindo `f64`), retornando
    /// `Err(CompareError::Incomparable)` se `val`, `min_v` ou `max_v` forem `NaN` ou `min_v > max_v`.
    pub fn try_clamp<T: PartialOrd>(val: T, min_v: T, max_v: T) -> Result<T, CompareError> {
        match min_v.partial_cmp(&max_v) {
            Some(Ordering::Greater) | None => return Err(CompareError::Incomparable),
            _ => {}
        }
        if val.partial_cmp(&val).is_none() {
            return Err(CompareError::Incomparable);
        }
        if val < min_v {
            Ok(min_v)
        } else if val > max_v {
            Ok(max_v)
        } else {
            Ok(val)
        }
    }

    /// Produtório equivalente a `math.prod(iter)` — alias direto para `arithmetic::mult`.
    #[inline]
    pub fn product<T: super::arithmetic::NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> T {
        super::arithmetic::mult(iter)
    }

    /// Verifica se `target` está contido na fatia (`target in seq`).
    #[inline]
    pub fn contains_in<T: PartialEq>(seq: &[T], target: &T) -> bool {
        seq.contains(target)
    }

    /// Conta quantas vezes `target` aparece na fatia (`seq.count(target)`).
    #[inline]
    pub fn count_in<T: PartialEq>(seq: &[T], target: &T) -> usize {
        seq.iter().filter(|&x| x == target).count()
    }

    /// Retorna o índice da primeira ocorrência de `target` na fatia (`seq.index(target)`).
    #[inline]
    pub fn find_in<T: PartialEq>(seq: &[T], target: &T) -> Option<usize> {
        seq.iter().position(|x| x == target)
    }
}

// ============================================================================
// 5) Aritmética — segura contra overflow/divisão por zero + PyInt/PyFloat
// ============================================================================
pub mod arithmetic {
    //! Aritmética genérica e segura para as larguras numéricas nativas da `std`
    //! (`i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f32`, `f64`).
    //!
    //! **Nota sobre `std`:** Rust utiliza inteiros de largura fixa (sem `BigInt` nativo).
    //! Para controle de overflow, utilize as funções `checked_*`, `saturating_*` ou `wrapping_*`.
    //!
    //! **Diferença `divmod` vs `py_divmod`:** [`divmod`] preserva a divisão truncada da v1.5.1
    //! (`-7 / 3 == -2, -7 % 3 == -1`), enquanto [`py_divmod`] e [`floor_div`] aplicam a divisão
    //! *floored* do Python (`py_divmod(-7, 3) == Ok((-3, 2))`).

    use std::convert::TryFrom;
    use std::fmt;

    /// Erros aritméticos explícitos.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum ArithError {
        DivisaoPorZero,
        RaizDeNegativoComIndicePar,
        IndiceDeRaizInvalido,
        Overflow,
        DominioInvalido,
        ConversaoInvalida,
    }

    impl fmt::Display for ArithError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ArithError::DivisaoPorZero => write!(f, "ZeroDivisionError: divisão por zero"),
                ArithError::RaizDeNegativoComIndicePar => {
                    write!(f, "ValueError: raiz real inválida de número negativo")
                }
                ArithError::IndiceDeRaizInvalido => {
                    write!(f, "ValueError: índice de raiz inválido (zero, NaN ou infinito)")
                }
                ArithError::Overflow => write!(f, "OverflowError: estouro de representação numérica"),
                ArithError::DominioInvalido => write!(f, "ValueError: valor fora do domínio matemático"),
                ArithError::ConversaoInvalida => {
                    write!(f, "ValueError: conversão numérica fora do intervalo de destino")
                }
            }
        }
    }

    impl std::error::Error for ArithError {}

    /// Trait numérica unificada preservada da v1.5.1 para `sum`, `sub`, `mult`, `div`, `divmod`,
    /// `pow`, `abs` e [`Matrix`](super::matrix_utils::Matrix).
    pub trait NumeroPy:
        Copy
        + PartialEq
        + PartialOrd
        + std::ops::Add<Output = Self>
        + std::ops::Sub<Output = Self>
        + std::ops::Mul<Output = Self>
        + std::ops::Div<Output = Self>
        + std::ops::Rem<Output = Self>
    {
        const ZERO: Self;
        const UM: Self;
        fn abs_val(self) -> Self;

        /// Divisão segura que também captura overflow de inteiros assinados (`MIN / -1`).
        fn checked_div_val(self, rhs: Self) -> Result<Self, ArithError> {
            if rhs == Self::ZERO {
                Err(ArithError::DivisaoPorZero)
            } else {
                Ok(self / rhs)
            }
        }

        /// Resto seguro que também captura overflow de inteiros assinados (`MIN % -1`).
        fn checked_rem_val(self, rhs: Self) -> Result<Self, ArithError> {
            if rhs == Self::ZERO {
                Err(ArithError::DivisaoPorZero)
            } else {
                Ok(self % rhs)
            }
        }
    }

    macro_rules! impl_numero_py_signed_int {
        ($($tipo:ty);+ $(;)?) => {
            $(
                impl NumeroPy for $tipo {
                    const ZERO: Self = 0;
                    const UM: Self = 1;
                    #[inline]
                    fn abs_val(self) -> Self {
                        if self < 0 { 0 - self } else { self }
                    }
                    #[inline]
                    fn checked_div_val(self, rhs: Self) -> Result<Self, ArithError> {
                        if rhs == 0 {
                            Err(ArithError::DivisaoPorZero)
                        } else {
                            self.checked_div(rhs).ok_or(ArithError::Overflow)
                        }
                    }
                    #[inline]
                    fn checked_rem_val(self, rhs: Self) -> Result<Self, ArithError> {
                        if rhs == 0 {
                            Err(ArithError::DivisaoPorZero)
                        } else {
                            self.checked_rem(rhs).ok_or(ArithError::Overflow)
                        }
                    }
                }
            )+
        };
    }

    macro_rules! impl_numero_py_unsigned_int {
        ($($tipo:ty);+ $(;)?) => {
            $(
                impl NumeroPy for $tipo {
                    const ZERO: Self = 0;
                    const UM: Self = 1;
                    #[inline]
                    fn abs_val(self) -> Self {
                        self
                    }
                }
            )+
        };
    }

    macro_rules! impl_numero_py_float {
        ($($tipo:ty);+ $(;)?) => {
            $(
                impl NumeroPy for $tipo {
                    const ZERO: Self = 0.0;
                    const UM: Self = 1.0;
                    #[inline]
                    fn abs_val(self) -> Self {
                        self.abs()
                    }
                }
            )+
        };
    }

    impl_numero_py_signed_int!(i8; i16; i32; i64; i128; isize);
    impl_numero_py_unsigned_int!(u8; u16; u32; u64; u128; usize);
    impl_numero_py_float!(f32; f64);

    /// `abs(x)` — valor absoluto genérico. Em floats, normaliza `-0.0` para `+0.0`.
    /// Nota: para `i8..i128` no valor mínimo (`MIN`), pode sofrer overflow em debug; prefira
    /// [`checked_abs`] quando a entrada for não confiável.
    #[inline]
    pub fn abs<T: NumeroPy>(x: T) -> T {
        x.abs_val()
    }

    /// `sum(iter)` — soma todos os elementos; devolve `0` se vazio (`sum([]) == 0`).
    #[inline]
    pub fn sum<T: NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> T {
        iter.into_iter().fold(T::ZERO, |acc, x| acc + x)
    }

    /// `sub(iter)` — subtração encadeada (`primeiro - segundo - ...`); `None` se vazio.
    pub fn sub<T: NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> Option<T> {
        let mut it = iter.into_iter();
        let primeiro = it.next()?;
        Some(it.fold(primeiro, |acc, x| acc - x))
    }

    /// `mult(iter)` — produtório (`math.prod` do Python); devolve `1` se vazio.
    #[inline]
    pub fn mult<T: NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> T {
        iter.into_iter().fold(T::UM, |acc, x| acc * x)
    }

    /// `div(a, b)` — divisão segura que devolve `Err(ArithError::DivisaoPorZero)` se `b == 0`
    /// e `Err(ArithError::Overflow)` em divisão assinada `T::MIN / -1`.
    #[inline]
    pub fn div<T: NumeroPy>(a: T, b: T) -> Result<T, ArithError> {
        a.checked_div_val(b)
    }

    /// `divmod(a, b)` — devolve `(a / b, a % b)` com proteção contra divisão por zero e
    /// `T::MIN / -1` (`Err(ArithError::Overflow)`).
    ///
    /// Preserva a divisão truncada da v1.5.1 por compatibilidade; para semântica exata de divisão
    /// *floored* do Python (ex.: `py_divmod(-7, 3) == Ok((-3, 2))`), utilize [`py_divmod`].
    #[inline]
    pub fn divmod<T: NumeroPy>(a: T, b: T) -> Result<(T, T), ArithError> {
        Ok((a.checked_div_val(b)?, a.checked_rem_val(b)?))
    }

    /// `pow(base, expoente)` — exponenciação rápida por quadratura em $O(\log(\text{expoente}))$.
    #[inline]
    pub fn pow<T: NumeroPy>(mut base: T, mut expoente: u32) -> T {
        let mut resultado = T::UM;
        while expoente > 0 {
            if expoente & 1 == 1 {
                resultado = resultado * base;
            }
            expoente >>= 1;
            if expoente > 0 {
                base = base * base;
            }
        }
        resultado
    }

    /// `root(x, indice)` — raiz n-ésima real (`f64`).
    ///
    /// Rejeita explicitamente:
    /// - `indice == 0.0` ou `!indice.is_finite()` (`NaN`, `±inf`) -> `IndiceDeRaizInvalido`
    /// - `x.is_nan()` -> `DominioInvalido`
    /// - `x == 0.0 && indice < 0.0` -> `DivisaoPorZero`
    /// - `x < 0.0` com índice não inteiro ou par (`indice.rem_euclid(2.0) != 1.0`) -> `RaizDeNegativoComIndicePar`
    pub fn root(x: f64, indice: f64) -> Result<f64, ArithError> {
        if indice == 0.0 || !indice.is_finite() {
            return Err(ArithError::IndiceDeRaizInvalido);
        }
        if x.is_nan() {
            return Err(ArithError::DominioInvalido);
        }
        if x == 0.0 && indice < 0.0 {
            return Err(ArithError::DivisaoPorZero);
        }
        if x < 0.0 {
            if indice.fract() != 0.0 || indice.rem_euclid(2.0) != 1.0 {
                return Err(ArithError::RaizDeNegativoComIndicePar);
            }
            return Ok(-((-x).powf(1.0 / indice)));
        }
        Ok(x.powf(1.0 / indice))
    }

    /// Trait com operações aritméticas controladas (`checked_*`, `saturating_*`, `wrapping_*`,
    /// `floor_div`, `py_divmod`, `ceil_div`, `gcd`, `lcm`) para todos os tipos inteiros da `std`.
    pub trait PyInt: NumeroPy + Eq + Ord {
        fn py_checked_add(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_sub(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_mul(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_div(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_rem(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_abs(self) -> Result<Self, ArithError>;
        fn py_checked_pow(self, exp: u32) -> Result<Self, ArithError>;
        fn py_saturating_add(self, rhs: Self) -> Self;
        fn py_saturating_sub(self, rhs: Self) -> Self;
        fn py_saturating_mul(self, rhs: Self) -> Self;
        fn py_saturating_pow(self, exp: u32) -> Self;
        fn py_wrapping_add(self, rhs: Self) -> Self;
        fn py_wrapping_sub(self, rhs: Self) -> Self;
        fn py_wrapping_mul(self, rhs: Self) -> Self;
        fn py_wrapping_pow(self, exp: u32) -> Self;
        fn py_divmod(self, rhs: Self) -> Result<(Self, Self), ArithError>;
        fn py_floor_div(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_ceil_div(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_gcd(self, rhs: Self) -> Result<Self, ArithError>;
        fn py_checked_lcm(self, rhs: Self) -> Result<Self, ArithError>;
    }

    fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            let r = a % b;
            a = b;
            b = r;
        }
        a
    }

    macro_rules! impl_py_int_signed {
        ($($tipo:ty);+ $(;)?) => {
            $(
                impl PyInt for $tipo {
                    #[inline]
                    fn py_checked_add(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_add(rhs).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_sub(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_sub(rhs).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_mul(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_mul(rhs).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_div(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_div_val(rhs)
                    }
                    #[inline]
                    fn py_checked_rem(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_rem_val(rhs)
                    }
                    #[inline]
                    fn py_checked_abs(self) -> Result<Self, ArithError> {
                        self.checked_abs().ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_pow(self, exp: u32) -> Result<Self, ArithError> {
                        self.checked_pow(exp).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_saturating_add(self, rhs: Self) -> Self { self.saturating_add(rhs) }
                    #[inline]
                    fn py_saturating_sub(self, rhs: Self) -> Self { self.saturating_sub(rhs) }
                    #[inline]
                    fn py_saturating_mul(self, rhs: Self) -> Self { self.saturating_mul(rhs) }
                    #[inline]
                    fn py_saturating_pow(self, exp: u32) -> Self { self.saturating_pow(exp) }
                    #[inline]
                    fn py_wrapping_add(self, rhs: Self) -> Self { self.wrapping_add(rhs) }
                    #[inline]
                    fn py_wrapping_sub(self, rhs: Self) -> Self { self.wrapping_sub(rhs) }
                    #[inline]
                    fn py_wrapping_mul(self, rhs: Self) -> Self { self.wrapping_mul(rhs) }
                    #[inline]
                    fn py_wrapping_pow(self, exp: u32) -> Self { self.wrapping_pow(exp) }

                    fn py_divmod(self, rhs: Self) -> Result<(Self, Self), ArithError> {
                        let q = self.checked_div_val(rhs)?;
                        let r = self % rhs;
                        if r != 0 && ((r < 0) != (rhs < 0)) {
                            Ok((q - 1, r + rhs))
                        } else {
                            Ok((q, r))
                        }
                    }

                    #[inline]
                    fn py_floor_div(self, rhs: Self) -> Result<Self, ArithError> {
                        Ok(self.py_divmod(rhs)?.0)
                    }

                    fn py_ceil_div(self, rhs: Self) -> Result<Self, ArithError> {
                        let q = self.checked_div_val(rhs)?;
                        let r = self % rhs;
                        if r != 0 && ((self > 0 && rhs > 0) || (self < 0 && rhs < 0)) {
                            Ok(q + 1)
                        } else {
                            Ok(q)
                        }
                    }

                    fn py_checked_gcd(self, rhs: Self) -> Result<Self, ArithError> {
                        let g = gcd_u128(self.unsigned_abs() as u128, rhs.unsigned_abs() as u128);
                        Self::try_from(g).map_err(|_| ArithError::Overflow)
                    }

                    fn py_checked_lcm(self, rhs: Self) -> Result<Self, ArithError> {
                        if self == 0 || rhs == 0 {
                            return Ok(0);
                        }
                        let a = self.unsigned_abs() as u128;
                        let b = rhs.unsigned_abs() as u128;
                        let g = gcd_u128(a, b);
                        let res = (a / g)
                            .checked_mul(b)
                            .ok_or(ArithError::Overflow)?;
                        Self::try_from(res).map_err(|_| ArithError::Overflow)
                    }
                }
            )+
        };
    }

    macro_rules! impl_py_int_unsigned {
        ($($tipo:ty);+ $(;)?) => {
            $(
                impl PyInt for $tipo {
                    #[inline]
                    fn py_checked_add(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_add(rhs).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_sub(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_sub(rhs).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_mul(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_mul(rhs).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_checked_div(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_div_val(rhs)
                    }
                    #[inline]
                    fn py_checked_rem(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_rem_val(rhs)
                    }
                    #[inline]
                    fn py_checked_abs(self) -> Result<Self, ArithError> {
                        Ok(self)
                    }
                    #[inline]
                    fn py_checked_pow(self, exp: u32) -> Result<Self, ArithError> {
                        self.checked_pow(exp).ok_or(ArithError::Overflow)
                    }
                    #[inline]
                    fn py_saturating_add(self, rhs: Self) -> Self { self.saturating_add(rhs) }
                    #[inline]
                    fn py_saturating_sub(self, rhs: Self) -> Self { self.saturating_sub(rhs) }
                    #[inline]
                    fn py_saturating_mul(self, rhs: Self) -> Self { self.saturating_mul(rhs) }
                    #[inline]
                    fn py_saturating_pow(self, exp: u32) -> Self { self.saturating_pow(exp) }
                    #[inline]
                    fn py_wrapping_add(self, rhs: Self) -> Self { self.wrapping_add(rhs) }
                    #[inline]
                    fn py_wrapping_sub(self, rhs: Self) -> Self { self.wrapping_sub(rhs) }
                    #[inline]
                    fn py_wrapping_mul(self, rhs: Self) -> Self { self.wrapping_mul(rhs) }
                    #[inline]
                    fn py_wrapping_pow(self, exp: u32) -> Self { self.wrapping_pow(exp) }

                    #[inline]
                    fn py_divmod(self, rhs: Self) -> Result<(Self, Self), ArithError> {
                        if rhs == 0 {
                            Err(ArithError::DivisaoPorZero)
                        } else {
                            Ok((self / rhs, self % rhs))
                        }
                    }

                    #[inline]
                    fn py_floor_div(self, rhs: Self) -> Result<Self, ArithError> {
                        self.checked_div_val(rhs)
                    }

                    #[inline]
                    fn py_ceil_div(self, rhs: Self) -> Result<Self, ArithError> {
                        if rhs == 0 {
                            return Err(ArithError::DivisaoPorZero);
                        }
                        let q = self / rhs;
                        let r = self % rhs;
                        Ok(if r != 0 { q + 1 } else { q })
                    }

                    #[inline]
                    fn py_checked_gcd(self, rhs: Self) -> Result<Self, ArithError> {
                        let g = gcd_u128(self as u128, rhs as u128);
                        Ok(g as Self)
                    }

                    fn py_checked_lcm(self, rhs: Self) -> Result<Self, ArithError> {
                        if self == 0 || rhs == 0 {
                            return Ok(0);
                        }
                        let a = self as u128;
                        let b = rhs as u128;
                        let g = gcd_u128(a, b);
                        let res = (a / g).checked_mul(b).ok_or(ArithError::Overflow)?;
                        Self::try_from(res).map_err(|_| ArithError::Overflow)
                    }
                }
            )+
        };
    }

    impl_py_int_signed!(i8; i16; i32; i64; i128; isize);
    impl_py_int_unsigned!(u8; u16; u32; u64; u128; usize);

    // Funções livres ergonômicas de aritmética controlada
    #[inline]
    pub fn checked_add<T: PyInt>(a: T, b: T) -> Result<T, ArithError> { a.py_checked_add(b) }
    #[inline]
    pub fn checked_sub<T: PyInt>(a: T, b: T) -> Result<T, ArithError> { a.py_checked_sub(b) }
    #[inline]
    pub fn checked_mul<T: PyInt>(a: T, b: T) -> Result<T, ArithError> { a.py_checked_mul(b) }
    #[inline]
    pub fn checked_div<T: PyInt>(a: T, b: T) -> Result<T, ArithError> { a.py_checked_div(b) }
    #[inline]
    pub fn checked_rem<T: PyInt>(a: T, b: T) -> Result<T, ArithError> { a.py_checked_rem(b) }
    #[inline]
    pub fn checked_abs<T: PyInt>(a: T) -> Result<T, ArithError> { a.py_checked_abs() }
    #[inline]
    pub fn checked_pow<T: PyInt>(a: T, exp: u32) -> Result<T, ArithError> { a.py_checked_pow(exp) }

    /// Soma uma sequência retornando `Err(ArithError::Overflow)` em caso de estouro.
    pub fn checked_sum<T: PyInt, I: IntoIterator<Item = T>>(iter: I) -> Result<T, ArithError> {
        let mut acc = T::ZERO;
        for x in iter {
            acc = acc.py_checked_add(x)?;
        }
        Ok(acc)
    }

    /// Multiplica uma sequência retornando `Err(ArithError::Overflow)` em caso de estouro.
    pub fn checked_mult<T: PyInt, I: IntoIterator<Item = T>>(iter: I) -> Result<T, ArithError> {
        let mut acc = T::UM;
        for x in iter {
            acc = acc.py_checked_mul(x)?;
        }
        Ok(acc)
    }

    #[inline]
    pub fn saturating_add<T: PyInt>(a: T, b: T) -> T { a.py_saturating_add(b) }
    #[inline]
    pub fn saturating_sub<T: PyInt>(a: T, b: T) -> T { a.py_saturating_sub(b) }
    #[inline]
    pub fn saturating_mul<T: PyInt>(a: T, b: T) -> T { a.py_saturating_mul(b) }
    #[inline]
    pub fn saturating_pow<T: PyInt>(a: T, exp: u32) -> T { a.py_saturating_pow(exp) }

    #[inline]
    pub fn wrapping_add<T: PyInt>(a: T, b: T) -> T { a.py_wrapping_add(b) }
    #[inline]
    pub fn wrapping_sub<T: PyInt>(a: T, b: T) -> T { a.py_wrapping_sub(b) }
    #[inline]
    pub fn wrapping_mul<T: PyInt>(a: T, b: T) -> T { a.py_wrapping_mul(b) }
    #[inline]
    pub fn wrapping_pow<T: PyInt>(a: T, exp: u32) -> T { a.py_wrapping_pow(exp) }

    /// Divisão *floored* estilo Python (`a // b`), sempre arredondando em direção a $-\infty$.
    #[inline]
    pub fn floor_div<T: PyInt>(a: T, b: T) -> Result<T, ArithError> {
        a.py_floor_div(b)
    }

    /// `divmod(a, b)` exato do Python: `(q, r)` com divisão *floored* (`q * b + r == a` e sinal de `r`
    /// acompanha `b`).
    #[inline]
    pub fn py_divmod<T: PyInt>(a: T, b: T) -> Result<(T, T), ArithError> {
        a.py_divmod(b)
    }

    /// Divisão inteira arredondada em direção a $+\infty$ (`ceil(a / b)`).
    #[inline]
    pub fn ceil_div<T: PyInt>(a: T, b: T) -> Result<T, ArithError> {
        a.py_ceil_div(b)
    }

    /// Máximo divisor comum (`math.gcd`). Retorna sempre valor não negativo.
    ///
    /// # Panics
    /// Causa panic se o resultado positivo não couber no tipo (ex.: `gcd(i64::MIN, 0)`). Para
    /// versão sem panic, use [`checked_gcd`].
    #[inline]
    pub fn gcd<T: PyInt>(a: T, b: T) -> T {
        a.py_checked_gcd(b)
            .expect("OverflowError: gcd excedeu o intervalo representável do tipo")
    }

    /// Versão sem panic de [`gcd`].
    #[inline]
    pub fn checked_gcd<T: PyInt>(a: T, b: T) -> Result<T, ArithError> {
        a.py_checked_gcd(b)
    }

    /// Mínimo múltiplo comum (`math.lcm`) com verificação contra overflow.
    #[inline]
    pub fn checked_lcm<T: PyInt>(a: T, b: T) -> Result<T, ArithError> {
        a.py_checked_lcm(b)
    }

    /// Mínimo múltiplo comum (`math.lcm`). Causa panic em overflow; use [`checked_lcm`] para `Result`.
    #[inline]
    pub fn lcm<T: PyInt>(a: T, b: T) -> T {
        checked_lcm(a, b).expect("OverflowError: lcm excedeu o intervalo representável do tipo")
    }

    /// Operações de arredondamento para `f32` e `f64`.
    pub trait PyFloat: NumeroPy {
        fn py_ceil(self) -> Self;
        fn py_floor(self) -> Self;
        fn py_round(self) -> Self;
        fn py_trunc(self) -> Self;
        fn py_is_finite(self) -> bool;
    }

    impl PyFloat for f32 {
        #[inline]
        fn py_ceil(self) -> Self { self.ceil() }
        #[inline]
        fn py_floor(self) -> Self { self.floor() }
        #[inline]
        fn py_round(self) -> Self { self.round() }
        #[inline]
        fn py_trunc(self) -> Self { self.trunc() }
        #[inline]
        fn py_is_finite(self) -> bool { self.is_finite() }
    }

    impl PyFloat for f64 {
        #[inline]
        fn py_ceil(self) -> Self { self.ceil() }
        #[inline]
        fn py_floor(self) -> Self { self.floor() }
        #[inline]
        fn py_round(self) -> Self { self.round() }
        #[inline]
        fn py_trunc(self) -> Self { self.trunc() }
        #[inline]
        fn py_is_finite(self) -> bool { self.is_finite() }
    }

    #[inline]
    pub fn ceil<T: PyFloat>(x: T) -> T { x.py_ceil() }
    #[inline]
    pub fn floor<T: PyFloat>(x: T) -> T { x.py_floor() }
    /// Nota: `round` em Rust arredonda empates (`.5`) para longe de zero (`round_ties_away`),
    /// diferindo do *banker's rounding* (`round_ties_even`) do Python.
    #[inline]
    pub fn round<T: PyFloat>(x: T) -> T { x.py_round() }
    #[inline]
    pub fn trunc<T: PyFloat>(x: T) -> T { x.py_trunc() }

    /// Equivalente a `math.isclose(a, b, rel_tol, abs_tol)` do Python.
    pub fn is_close(a: f64, b: f64, rel_tol: f64, abs_tol: f64) -> bool {
        if a == b {
            return true;
        }
        if a.is_nan() || b.is_nan() || !a.is_finite() || !b.is_finite() {
            return false;
        }
        let diff = (a - b).abs();
        diff <= (rel_tol * a.abs().max(b.abs())).max(abs_tol)
    }

    /// Conversão segura entre tipos inteiros usando `TryFrom` (sem truncamento silencioso de `as`).
    #[inline]
    pub fn try_cast<F, T>(valor: F) -> Result<T, ArithError>
    where
        T: TryFrom<F>,
    {
        T::try_from(valor).map_err(|_| ArithError::ConversaoInvalida)
    }

    /// Converte `f64` (parte inteira truncada) para `i64` rejeitando `NaN`, `±inf` e valores fora
    /// do intervalo exato `[-2^63, 2^63 - 1]` sem saturação silenciosa.
    pub fn f64_to_i64_checked(v: f64) -> Result<i64, ArithError> {
        if !v.is_finite() {
            return Err(ArithError::ConversaoInvalida);
        }
        let t = v.trunc();
        // i64::MIN == -9223372036854775808.0 (exato em f64).
        // 2^63 == 9223372036854775808.0 já ultrapassa i64::MAX!
        if (-9223372036854775808.0..9223372036854775808.0).contains(&t) {
            Ok(t as i64)
        } else {
            Err(ArithError::ConversaoInvalida)
        }
    }
}

// ============================================================================
// 6) Strings — zero-copy onde possível + Unicode explícito
// ============================================================================
pub mod string_utils {
    //! Manipulação de strings com foco em empréstimo zero-copy (`&str`) onde possível e
    //! alocação única nas operações que geram texto novo.
    //!
    //! **Nota sobre Unicode na `std`:**
    //! - Índices em [`find`], [`rfind`], [`char_count`], [`substring`] e [`substring_py`] operam em
    //!   **escalares Unicode (`char`)** — equivalentes ao modelo de `str` do Python — e **não**
    //!   em bytes UTF-8 nem em *grapheme clusters* (sequências combinantes / emojis ZWJ).
    //! - Para posições em bytes UTF-8 (custo $O(1)$), utilize [`find_byte`] / [`byte_len`].
    //! - A `std` não fornece normalização Unicode (NFC/NFD/NFKC) nem segmentação por grapheme.

    /// `split(s, sep)` — fatias emprestadas de `s` em `Vec<&str>` (zero alocação de caracteres).
    #[inline]
    pub fn split<'a>(s: &'a str, sep: &str) -> Vec<&'a str> {
        s.split(sep).collect()
    }

    /// Variante lazy de [`split`] que não aloca `Vec`.
    #[inline]
    pub fn split_lazy<'a>(s: &'a str, sep: &'a str) -> impl Iterator<Item = &'a str> {
        s.split(sep)
    }

    /// `s.split()` sem separador — divide por qualquer sequência de espaços em branco (lazy).
    #[inline]
    pub fn split_whitespace(s: &str) -> std::str::SplitWhitespace<'_> {
        s.split_whitespace()
    }

    /// `s.rsplit(sep)` — divide da direita para a esquerda em `Vec<&str>`.
    #[inline]
    pub fn rsplit<'a>(s: &'a str, sep: &str) -> Vec<&'a str> {
        s.rsplit(sep).collect()
    }

    /// Variante lazy de [`rsplit`].
    #[inline]
    pub fn rsplit_lazy<'a>(s: &'a str, sep: &'a str) -> impl Iterator<Item = &'a str> {
        s.rsplit(sep)
    }

    /// `s.splitlines()` — divide por `\n` e `\r\n` removendo terminadores.
    #[inline]
    pub fn splitlines(s: &str) -> Vec<&str> {
        s.lines().collect()
    }

    /// Variante lazy de [`splitlines`].
    #[inline]
    pub fn splitlines_lazy(s: &str) -> std::str::Lines<'_> {
        s.lines()
    }

    /// `s.partition(sep)` — divide na **primeira** ocorrência de `sep`, devolvendo
    /// `(antes, sep, depois)` ou `(s, "", "")` se ausente (zero-copy).
    #[inline]
    pub fn partition<'a>(s: &'a str, sep: &'a str) -> (&'a str, &'a str, &'a str) {
        match s.split_once(sep) {
            Some((a, b)) => (a, &s[a.len()..a.len() + sep.len()], b),
            None => (s, "", ""),
        }
    }

    /// `s.rpartition(sep)` — divide na **última** ocorrência de `sep`, devolvendo
    /// `(antes, sep, depois)` ou `("", "", s)` se ausente (zero-copy).
    #[inline]
    pub fn rpartition<'a>(s: &'a str, sep: &'a str) -> (&'a str, &'a str, &'a str) {
        match s.rsplit_once(sep) {
            Some((a, b)) => (a, &s[a.len()..a.len() + sep.len()], b),
            None => ("", "", s),
        }
    }

    /// `strip(s)` — remove espaços em branco do início e do fim sem alocar.
    #[inline]
    pub fn strip(s: &str) -> &str {
        s.trim()
    }

    /// `lstrip(s)` — remove espaços em branco do início sem alocar.
    #[inline]
    pub fn lstrip(s: &str) -> &str {
        s.trim_start()
    }

    /// `rstrip(s)` — remove espaços em branco do fim sem alocar.
    #[inline]
    pub fn rstrip(s: &str) -> &str {
        s.trim_end()
    }

    /// `s.strip(chars)` — remove quaisquer caracteres contidos em `chars` das bordas (zero-copy).
    #[inline]
    pub fn strip_chars<'a>(s: &'a str, chars: &[char]) -> &'a str {
        s.trim_matches(chars)
    }

    /// `s.lstrip(chars)` — remove caracteres de `chars` do início (zero-copy).
    #[inline]
    pub fn lstrip_chars<'a>(s: &'a str, chars: &[char]) -> &'a str {
        s.trim_start_matches(chars)
    }

    /// `s.rstrip(chars)` — remove caracteres de `chars` do fim (zero-copy).
    #[inline]
    pub fn rstrip_chars<'a>(s: &'a str, chars: &[char]) -> &'a str {
        s.trim_end_matches(chars)
    }

    /// `s.removeprefix(prefix)` — remove `prefix` no máximo uma vez (zero-copy).
    #[inline]
    pub fn removeprefix<'a>(s: &'a str, prefix: &str) -> &'a str {
        s.strip_prefix(prefix).unwrap_or(s)
    }

    /// `s.removesuffix(suffix)` — remove `suffix` no máximo uma vez (zero-copy).
    #[inline]
    pub fn removesuffix<'a>(s: &'a str, suffix: &str) -> &'a str {
        s.strip_suffix(suffix).unwrap_or(s)
    }

    /// `join(sep, iter)` — equivalente a `sep.join(iter)` do Python.
    /// Faz uma única alocação protegida contra overflow em `size_hint`.
    pub fn join<I, S>(sep: &str, iter: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut it = iter.into_iter();
        let primeiro = match it.next() {
            Some(p) => p,
            None => return String::new(),
        };
        let (min_len, _) = it.size_hint();
        let estimado = primeiro
            .as_ref()
            .len()
            .saturating_add(min_len.min(4096).saturating_mul(sep.len().saturating_add(4)));
        let mut resultado = String::with_capacity(estimado);
        resultado.push_str(primeiro.as_ref());
        for item in it {
            resultado.push_str(sep);
            resultado.push_str(item.as_ref());
        }
        resultado
    }

    /// `replace(s, de, para)` — equivalente a `s.replace(de, para)` do Python.
    #[inline]
    pub fn replace(s: &str, de: &str, para: &str) -> String {
        s.replace(de, para)
    }

    /// `lower(s)` — equivalente a `s.lower()` do Python.
    #[inline]
    pub fn lower(s: &str) -> String {
        s.to_lowercase()
    }

    /// `upper(s)` — equivalente a `s.upper()` do Python.
    #[inline]
    pub fn upper(s: &str) -> String {
        s.to_uppercase()
    }

    /// `startswith(s, prefixo)` — equivalente a `s.startswith(prefixo)`.
    #[inline]
    pub fn startswith(s: &str, prefixo: &str) -> bool {
        s.starts_with(prefixo)
    }

    /// `endswith(s, sufixo)` — equivalente a `s.endswith(sufixo)`.
    #[inline]
    pub fn endswith(s: &str, sufixo: &str) -> bool {
        s.ends_with(sufixo)
    }

    /// `find(s, sub)` — devolve o índice **em caracteres Unicode** (igual ao
    /// `s.find(sub)` do Python) ou `-1` se não encontrar. Complexidade: $O(N)$.
    pub fn find(s: &str, sub: &str) -> i64 {
        match s.find(sub) {
            Some(byte_idx) => s[..byte_idx].chars().count() as i64,
            None => -1,
        }
    }

    /// Variante idiomática de [`find`] retornando `Option<usize>` (em `char`s).
    #[inline]
    pub fn find_opt(s: &str, sub: &str) -> Option<usize> {
        s.find(sub).map(|byte_idx| s[..byte_idx].chars().count())
    }

    /// Busca direta retornando o deslocamento em **bytes UTF-8** ($O(N)$ sem contar caracteres).
    #[inline]
    pub fn find_byte(s: &str, sub: &str) -> Option<usize> {
        s.find(sub)
    }

    /// `s.rfind(sub)` — índice **em caracteres Unicode** da última ocorrência ou `-1`.
    pub fn rfind(s: &str, sub: &str) -> i64 {
        match s.rfind(sub) {
            Some(byte_idx) => s[..byte_idx].chars().count() as i64,
            None => -1,
        }
    }

    /// Variante idiomática de [`rfind`] retornando `Option<usize>` (em `char`s).
    #[inline]
    pub fn rfind_opt(s: &str, sub: &str) -> Option<usize> {
        s.rfind(sub).map(|byte_idx| s[..byte_idx].chars().count())
    }

    /// Verifica se `sub` está contida em `s` (`sub in s`).
    #[inline]
    pub fn contains(s: &str, sub: &str) -> bool {
        s.contains(sub)
    }

    /// `count(s, sub)` — conta ocorrências não-sobrepostas de `sub` em `s` (`s.count(sub)`).
    #[inline]
    pub fn count(s: &str, sub: &str) -> usize {
        s.matches(sub).count()
    }

    /// `is_palindrome(s)` — verifica em $O(N)$ e zero alocação se `s` é palíndromo por `char`.
    #[inline]
    pub fn is_palindrome(s: &str) -> bool {
        let mut chars = s.chars();
        while let (Some(a), Some(b)) = (chars.next(), chars.next_back()) {
            if a != b {
                return false;
            }
        }
        true
    }

    /// `s.isascii()` — retorna `true` se vazia ou se todos os bytes forem ASCII (`0x00..=0x7F`).
    #[inline]
    pub fn is_ascii(s: &str) -> bool {
        s.is_ascii()
    }

    /// Equivalente a `s.isnumeric()` do Python: retorna `true` se `s` **não é vazia** e todos os
    /// caracteres satisfazem `char::is_numeric` (inclui dígitos decimais e caracteres numéricos
    /// Unicode como `²`, `¾`, algarismos romanos; para apenas `0..=9` ASCII use [`is_ascii_digits`]).
    #[inline]
    pub fn is_numeric(s: &str) -> bool {
        !s.is_empty() && s.chars().all(char::is_numeric)
    }

    /// Retorna `true` se `s` não for vazia e contiver apenas dígitos ASCII (`'0'..='9'`).
    #[inline]
    pub fn is_ascii_digits(s: &str) -> bool {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
    }

    /// Retorna `true` se `s` não for vazia e todos os caracteres forem alfabéticos (`char::is_alphabetic`).
    #[inline]
    pub fn is_alpha(s: &str) -> bool {
        !s.is_empty() && s.chars().all(char::is_alphabetic)
    }

    /// Retorna `true` se `s` não for vazia e todos os caracteres forem alfanuméricos (`char::is_alphanumeric`).
    #[inline]
    pub fn is_alnum(s: &str) -> bool {
        !s.is_empty() && s.chars().all(char::is_alphanumeric)
    }

    /// Retorna `true` se a string tem tamanho zero.
    #[inline]
    pub fn is_empty(s: &str) -> bool {
        s.is_empty()
    }

    /// Quantidade de caracteres Unicode (`char`), diferente de [`byte_len`].
    #[inline]
    pub fn char_count(s: &str) -> usize {
        s.chars().count()
    }

    /// Comprimento em bytes UTF-8 (`s.len()` nativo).
    #[inline]
    pub fn byte_len(s: &str) -> usize {
        s.len()
    }

    /// Fatia zero-copy por intervalo de caracteres Unicode `[start..end]` (clamped, sem panic).
    pub fn substring(s: &str, start: usize, end: usize) -> &str {
        if start >= end {
            return "";
        }
        let mut iter = s.char_indices();
        let ini_byte = match iter.nth(start) {
            Some((idx, _)) => idx,
            None => return "",
        };
        let rest_count = end - start;
        // Já consumimos o char na posição `start`; avançamos `rest_count - 1` chars à frente.
        let fim_byte = match iter.nth(rest_count - 1) {
            Some((idx, _)) => idx,
            None => s.len(),
        };
        &s[ini_byte..fim_byte]
    }

    /// Fatiamento Python `s[start:end]` em caracteres Unicode, com índices negativos e zero-copy.
    pub fn substring_py(s: &str, start: Option<isize>, end: Option<isize>) -> &str {
        let n = s.chars().count();
        let n_i = n as i128;
        let clamp = |b: isize| -> usize {
            let v = if b < 0 { n_i + (b as i128) } else { b as i128 };
            if v <= 0 {
                0
            } else if v >= n_i {
                n
            } else {
                v as usize
            }
        };
        let ini = start.map_or(0, clamp);
        let fim = end.map_or(n, clamp);
        substring(s, ini, fim)
    }

    /// `s.rjust(width, fill)` — preenche à esquerda com `fill` até atingir `width` caracteres.
    pub fn pad_left(s: &str, width: usize, fill: char) -> String {
        let len = s.chars().count();
        if len >= width {
            return s.to_string();
        }
        let falta = width - len;
        let mut out = String::with_capacity(s.len() + falta * fill.len_utf8());
        for _ in 0..falta {
            out.push(fill);
        }
        out.push_str(s);
        out
    }

    /// `s.ljust(width, fill)` — preenche à direita com `fill` até atingir `width` caracteres.
    pub fn pad_right(s: &str, width: usize, fill: char) -> String {
        let len = s.chars().count();
        if len >= width {
            return s.to_string();
        }
        let falta = width - len;
        let mut out = String::with_capacity(s.len() + falta * fill.len_utf8());
        out.push_str(s);
        for _ in 0..falta {
            out.push(fill);
        }
        out
    }

    /// `s.center(width, fill)` — centraliza a string preenchendo ambos os lados com `fill`.
    pub fn center(s: &str, width: usize, fill: char) -> String {
        let len = s.chars().count();
        if len >= width {
            return s.to_string();
        }
        let falta = width - len;
        let esq = falta / 2;
        let dir = falta - esq;
        let mut out = String::with_capacity(s.len() + falta * fill.len_utf8());
        for _ in 0..esq {
            out.push(fill);
        }
        out.push_str(s);
        for _ in 0..dir {
            out.push(fill);
        }
        out
    }
}

// ============================================================================
// 7) OOP estilo C++/Python — macro `class!` (composição + Deref/DerefMut)
// ============================================================================

/// Declara uma estrutura e seus métodos em um único bloco estilo C++/Python,
/// gerando construtor `new(...)` automático e suportando herança (`struct Filha : Pai`)
/// via composição (`pub base: Pai`) + `Deref`/`DerefMut`.
///
/// **Limitações técnicas documentadas:**
/// - `Deref`/`DerefMut` emulam herança de campos e métodos da classe pai via composição,
///   **não** fornecem despacho dinâmico (*virtual dispatch* / polimorfismo de sobrescrita):
///   um método da classe pai chamando `self.foo()` sempre invocará a implementação do pai,
///   não a eventual sobrescrita na filha. Para polimorfismo real em Rust, use `trait`s.
#[macro_export]
macro_rules! class {
    // 1) Classe base padrão
    (
        $(#[$meta:meta])*
        struct $nome:ident {
            $( $(#[$cmeta:meta])* $campo:ident : $tipo:ty ),* $(,)?
        }
        impl {
            $( $(#[$mmeta:meta])* fn $metodo:ident ( $($params:tt)* ) $(-> $ret:ty)? $corpo:block )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $nome {
            $( $(#[$cmeta])* pub $campo: $tipo ),*
        }

        impl $nome {
            /// Construtor gerado automaticamente a partir dos campos declarados.
            pub fn new( $( $campo: $tipo ),* ) -> Self {
                Self { $( $campo ),* }
            }

            $(
                $(#[$mmeta])*
                pub fn $metodo( $($params)* ) $(-> $ret)? $corpo
            )*
        }
    };

    // 2) Classe derivada com herança estilo C++ (`struct Filha : Pai { ... }`)
    (
        $(#[$meta:meta])*
        struct $nome:ident : $pai:ty {
            $( $(#[$cmeta:meta])* $campo:ident : $tipo:ty ),* $(,)?
        }
        impl {
            $( $(#[$mmeta:meta])* fn $metodo:ident ( $($params:tt)* ) $(-> $ret:ty)? $corpo:block )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $nome {
            pub base: $pai,
            $( $(#[$cmeta])* pub $campo: $tipo ),*
        }

        impl $nome {
            /// Construtor que recebe a instância da classe `base` seguida dos campos próprios.
            pub fn new(base: $pai, $( $campo: $tipo ),* ) -> Self {
                Self { base, $( $campo ),* }
            }

            /// Acesso explícito à instância pai subjacente (`super`).
            #[inline]
            pub fn as_base(&self) -> &$pai {
                &self.base
            }

            /// Acesso mutável explícito à instância pai subjacente (`super`).
            #[inline]
            pub fn as_base_mut(&mut self) -> &mut $pai {
                &mut self.base
            }

            $(
                $(#[$mmeta])*
                pub fn $metodo( $($params)* ) $(-> $ret)? $corpo
            )*
        }

        impl ::std::ops::Deref for $nome {
            type Target = $pai;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }

        impl ::std::ops::DerefMut for $nome {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.base
            }
        }
    };
}

// ============================================================================
// 8) Tempo — time(), sleep(), perf_counter(), timeit(), Timer, Deadline, UTC
// ============================================================================
pub mod time_utils {
    //! Equivalente (100% `std`) aos módulos `time`, `timeit` e `calendar` do Python,
    //! expandido com [`Timer`], [`Deadline`], validações estritas de data e cálculos em `i128`.
    //!
    //! **Limitação técnica da `std`:** todas as conversões de data/hora operam exclusivamente em
    //! **UTC** (calendário gregoriano proléptico). Suporte completo a fusos horários regionais
    //! (IANA tzdb) e horário de verão (DST) exige uma base externa ausente na `std`.

    use std::convert::TryFrom;
    use std::fmt::{self, Write as _};
    use std::hint::black_box;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    /// Erros explícitos de validação de tempo, durações e datas UTC.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TimeError {
        /// Duração negativa, `NaN` ou infinita.
        InvalidDuration,
        /// Data ou horário fora dos limites válidos (ex.: 30 de fevereiro, hora 25).
        InvalidDate(String),
        /// Timestamp resultante ultrapassou os limites de `i64` ou `SystemTime`.
        TimestampOverflow,
    }

    impl fmt::Display for TimeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                TimeError::InvalidDuration => {
                    write!(f, "ValueError: duração inválida (negativa, NaN ou infinita)")
                }
                TimeError::InvalidDate(msg) => write!(f, "ValueError: data inválida ({})", msg),
                TimeError::TimestampOverflow => {
                    write!(f, "OverflowError: timestamp fora do intervalo suportado")
                }
            }
        }
    }

    impl std::error::Error for TimeError {}

    /// `time.time()` — segundos desde 1970-01-01 00:00:00 UTC como `f64`.
    #[inline]
    pub fn time() -> f64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs_f64(),
            Err(e) => -e.duration().as_secs_f64(),
        }
    }

    /// Converte segundos (`f64`) para [`Duration`] rejeitando valores negativos, `NaN` ou infinitos.
    #[inline]
    pub fn duration_from_secs(secs: f64) -> Result<Duration, TimeError> {
        Duration::try_from_secs_f64(secs).map_err(|_| TimeError::InvalidDuration)
    }

    /// Converte [`Duration`] para segundos em `f64`.
    #[inline]
    pub fn duration_to_secs(d: Duration) -> f64 {
        d.as_secs_f64()
    }

    /// `time.sleep(secs)` — suspende a thread atual por `secs` segundos.
    ///
    /// # Panics
    /// Causa panic se `secs` for negativo, `NaN` ou infinito (`ValueError` no Python).
    /// Para tratamento sem panic, utilize [`try_sleep`].
    pub fn sleep(secs: f64) {
        let d = Duration::try_from_secs_f64(secs)
            .unwrap_or_else(|_| panic!("ValueError: sleep length inválido ({})", secs));
        std::thread::sleep(d);
    }

    /// Versão segura de [`sleep`] que devolve `Err(TimeError::InvalidDuration)` em valores
    /// inválidos sem causar panic.
    pub fn try_sleep(secs: f64) -> Result<(), TimeError> {
        let d = duration_from_secs(secs)?;
        std::thread::sleep(d);
        Ok(())
    }

    fn origem() -> Instant {
        static ORIGEM: OnceLock<Instant> = OnceLock::new();
        *ORIGEM.get_or_init(Instant::now)
    }

    /// `time.perf_counter()` — relógio monotônico de alta precisão em segundos (`f64`).
    #[inline]
    pub fn perf_counter() -> f64 {
        origem().elapsed().as_secs_f64()
    }

    /// `time.perf_counter_ns()` — relógio monotônico em nanossegundos (`u64`).
    #[inline]
    pub fn perf_counter_ns() -> u64 {
        origem().elapsed().as_nanos() as u64
    }

    /// Relógio monotônico em nanossegundos sem truncamento (`u128`).
    #[inline]
    pub fn perf_counter_ns_u128() -> u128 {
        origem().elapsed().as_nanos()
    }

    /// `timeit.timeit(f, number=n)` — executa `f` `number` vezes protegendo o
    /// resultado com `black_box` e devolve o tempo total em segundos.
    pub fn timeit<R, F: FnMut() -> R>(number: u32, mut f: F) -> f64 {
        let inicio = Instant::now();
        for _ in 0..number {
            black_box(f());
        }
        inicio.elapsed().as_secs_f64()
    }

    /// `timeit.repeat(f, repeat=r, number=n)` — executa `r` baterias de `n` execuções.
    pub fn timeit_repeat<R, F: FnMut() -> R>(repeat: u32, number: u32, mut f: F) -> Vec<f64> {
        (0..repeat).map(|_| timeit(number, &mut f)).collect()
    }

    /// Macro ergonômica de benchmark: `timeit!(1_000, { codigo() })` ou `timeit!({ codigo() })`.
    #[macro_export]
    macro_rules! timeit {
        ($body:block) => {
            $crate::time_utils::timeit(1, || $body)
        };
        ($n:expr, $body:block) => {
            $crate::time_utils::timeit($n, || $body)
        };
    }

    /// Cronômetro monotônico reutilizável com suporte a parciais (`lap`).
    #[derive(Debug, Clone, Copy)]
    pub struct Timer {
        inicio: Instant,
        ultima_volta: Instant,
    }

    impl Default for Timer {
        fn default() -> Self {
            Self::start()
        }
    }

    impl Timer {
        /// Inicia o cronômetro imediatamente.
        #[inline]
        pub fn start() -> Self {
            let agora = Instant::now();
            Timer {
                inicio: agora,
                ultima_volta: agora,
            }
        }

        /// Tempo decorrido desde `start` ou do último `reset`.
        #[inline]
        pub fn elapsed(&self) -> Duration {
            self.inicio.elapsed()
        }

        /// Segundos (`f64`) decorridos desde o início.
        #[inline]
        pub fn elapsed_secs(&self) -> f64 {
            self.inicio.elapsed().as_secs_f64()
        }

        /// Milissegundos (`f64`) decorridos desde o início.
        #[inline]
        pub fn elapsed_ms(&self) -> f64 {
            self.inicio.elapsed().as_secs_f64() * 1_000.0
        }

        /// Microssegundos decorridos.
        #[inline]
        pub fn elapsed_us(&self) -> u128 {
            self.inicio.elapsed().as_micros()
        }

        /// Nanossegundos decorridos.
        #[inline]
        pub fn elapsed_ns(&self) -> u128 {
            self.inicio.elapsed().as_nanos()
        }

        /// Reinicia o cronômetro para o instante atual.
        #[inline]
        pub fn reset(&mut self) {
            let agora = Instant::now();
            self.inicio = agora;
            self.ultima_volta = agora;
        }

        /// Devolve o tempo decorrido desde a última chamada de `lap` (ou `start`) e reinicia o marco da volta.
        #[inline]
        pub fn lap(&mut self) -> Duration {
            let agora = Instant::now();
            let d = agora.saturating_duration_since(self.ultima_volta);
            self.ultima_volta = agora;
            d
        }
    }

    /// Prazo limite monotônico (`Deadline`) para controle de timeouts compostos.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Deadline {
        alvo: Instant,
    }

    impl Deadline {
        /// Cria um `Deadline` daqui a `secs` segundos (erro se negativo/NaN/infinito ou overflow).
        pub fn from_secs(secs: f64) -> Result<Self, TimeError> {
            let d = duration_from_secs(secs)?;
            let alvo = Instant::now()
                .checked_add(d)
                .ok_or(TimeError::TimestampOverflow)?;
            Ok(Deadline { alvo })
        }

        /// Cria um `Deadline` após a `Duration` especificada.
        #[inline]
        pub fn from_duration(d: Duration) -> Result<Self, TimeError> {
            let alvo = Instant::now()
                .checked_add(d)
                .ok_or(TimeError::TimestampOverflow)?;
            Ok(Deadline { alvo })
        }

        /// Retorna `true` se o prazo limite já foi atingido ou ultrapassado.
        #[inline]
        pub fn is_expired(&self) -> bool {
            Instant::now() >= self.alvo
        }

        /// Tempo restante até o vencimento (`Duration::ZERO` se expirado).
        #[inline]
        pub fn remaining(&self) -> Duration {
            self.alvo.saturating_duration_since(Instant::now())
        }

        /// Tempo restante em segundos (`0.0` se expirado).
        #[inline]
        pub fn remaining_secs(&self) -> f64 {
            self.remaining().as_secs_f64()
        }
    }

    /// Retorna `true` se `year` é ano bissexto no calendário gregoriano proléptico.
    #[inline]
    pub const fn is_leap_year(year: i64) -> bool {
        (year.rem_euclid(4) == 0 && year.rem_euclid(100) != 0) || year.rem_euclid(400) == 0
    }

    /// Retorna o número de dias do mês (`1..=12`) no ano dado, ou `None` se mês inválido.
    #[inline]
    pub const fn days_in_month(year: i64, month: u32) -> Option<u32> {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
            4 | 6 | 9 | 11 => Some(30),
            2 => Some(if is_leap_year(year) { 29 } else { 28 }),
            _ => None,
        }
    }

    /// Equivalente a `time.struct_time` em UTC (`weekday`: 0 = segunda .. 6 = domingo).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StructTime {
        pub year: i64,
        pub month: u32,
        pub day: u32,
        pub hour: u32,
        pub minute: u32,
        pub second: u32,
        pub weekday: u32,
        pub yearday: u32,
    }

    fn days_from_civil(y: i64, m: u32, d: u32) -> i128 {
        let y = if m <= 2 { (y as i128) - 1 } else { y as i128 };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = if m > 2 { m as i128 - 3 } else { m as i128 + 9 };
        let doy = (153 * mp + 2) / 5 + d as i128 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    fn civil_from_days(z: i128) -> (i64, u32, u32) {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
        (y as i64, m, d)
    }

    impl StructTime {
        /// Constrói um `StructTime` UTC validando calendário (inclusive anos bissextos) e
        /// horário (`00:00:00..=23:59:59`, sem segundos bissextos POSIX). Preenche `weekday` e
        /// `yearday` automaticamente.
        pub fn new_utc(
            year: i64,
            month: u32,
            day: u32,
            hour: u32,
            minute: u32,
            second: u32,
        ) -> Result<Self, TimeError> {
            let max_dia = days_in_month(year, month).ok_or_else(|| {
                TimeError::InvalidDate(format!("mês inválido {}", month))
            })?;
            if day == 0 || day > max_dia {
                return Err(TimeError::InvalidDate(format!(
                    "dia {} inválido para {}-{:02}",
                    day, year, month
                )));
            }
            if hour >= 24 || minute >= 60 || second >= 60 {
                return Err(TimeError::InvalidDate(format!(
                    "horário inválido {:02}:{:02}:{:02}",
                    hour, minute, second
                )));
            }
            let dias = days_from_civil(year, month, day);
            let jan1 = days_from_civil(year, 1, 1);
            Ok(StructTime {
                year,
                month,
                day,
                hour,
                minute,
                second,
                weekday: (dias + 3).rem_euclid(7) as u32,
                yearday: (dias - jan1 + 1) as u32,
            })
        }

        /// Verifica se os campos de data e hora são válidos no calendário gregoriano.
        pub fn is_valid(&self) -> bool {
            match days_in_month(self.year, self.month) {
                Some(max_d) => {
                    self.day >= 1
                        && self.day <= max_d
                        && self.hour < 24
                        && self.minute < 60
                        && self.second < 60
                }
                None => false,
            }
        }

        /// Formata no padrão ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
        pub fn to_iso8601(&self) -> String {
            if (0..=9999).contains(&self.year) {
                format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    self.year, self.month, self.day, self.hour, self.minute, self.second
                )
            } else {
                format!(
                    "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    self.year, self.month, self.day, self.hour, self.minute, self.second
                )
            }
        }
    }

    /// `time.gmtime(secs)` — converte timestamp epoch em `StructTime` UTC sem overflow
    /// mesmo para `i64::MIN` e `i64::MAX`.
    pub fn gmtime(secs: i64) -> StructTime {
        let s = secs as i128;
        let dias = s.div_euclid(86_400);
        let resto = s.rem_euclid(86_400) as u32;
        let (year, month, day) = civil_from_days(dias);
        StructTime {
            year,
            month,
            day,
            hour: resto / 3_600,
            minute: (resto % 3_600) / 60,
            second: resto % 60,
            weekday: (dias + 3).rem_euclid(7) as u32,
            yearday: (dias - days_from_civil(year, 1, 1) + 1) as u32,
        }
    }

    /// `gmtime_now()` — devolve a data/hora UTC atual com precisão inteira (sem perda via `f64`).
    #[inline]
    pub fn gmtime_now() -> StructTime {
        let secs = system_time_to_unix_secs(SystemTime::now());
        gmtime(secs)
    }

    /// Converte um `SystemTime` para timestamp Unix em segundos inteiros (`i64`).
    pub fn system_time_to_unix_secs(t: SystemTime) -> i64 {
        match t.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs().min(i64::MAX as u64) as i64,
            Err(e) => {
                let d = e.duration();
                let extra = if d.subsec_nanos() > 0 { 1u64 } else { 0u64 };
                let mag = d.as_secs().saturating_add(extra).min(i64::MAX as u64);
                -(mag as i64)
            }
        }
    }

    /// Converte um timestamp Unix (`i64`) em [`SystemTime`], retornando erro em overflow.
    pub fn unix_secs_to_system_time(secs: i64) -> Result<SystemTime, TimeError> {
        if secs >= 0 {
            UNIX_EPOCH
                .checked_add(Duration::from_secs(secs as u64))
                .ok_or(TimeError::TimestampOverflow)
        } else {
            UNIX_EPOCH
                .checked_sub(Duration::from_secs(secs.unsigned_abs()))
                .ok_or(TimeError::TimestampOverflow)
        }
    }

    /// `calendar.timegm(t)` — converte `StructTime` UTC de volta para segundos epoch.
    /// Para validação rigorosa da data e checagem de overflow, utilize [`try_mktime_utc`].
    #[inline]
    pub fn mktime_utc(t: &StructTime) -> i64 {
        let s = days_from_civil(t.year, t.month, t.day) * 86_400
            + t.hour as i128 * 3_600
            + t.minute as i128 * 60
            + t.second as i128;
        s as i64
    }

    /// Versão validada de [`mktime_utc`] que rejeita datas/horários inválidos e overflow de `i64`.
    pub fn try_mktime_utc(t: &StructTime) -> Result<i64, TimeError> {
        if !t.is_valid() {
            return Err(TimeError::InvalidDate(format!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                t.year, t.month, t.day, t.hour, t.minute, t.second
            )));
        }
        let s = days_from_civil(t.year, t.month, t.day) * 86_400
            + t.hour as i128 * 3_600
            + t.minute as i128 * 60
            + t.second as i128;
        i64::try_from(s).map_err(|_| TimeError::TimestampOverflow)
    }

    const DIAS: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    const MESES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    /// `time.strftime(fmt, t)` — formata `StructTime` segundo as diretivas padrão C/Python.
    /// Protegido contra `month`/`weekday` fora de faixa quando construído manualmente.
    pub fn strftime(fmt: &str, t: &StructTime) -> String {
        let mut out = String::with_capacity(fmt.len() + 16);
        let mut chars = fmt.chars();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('Y') => {
                    if (0..=9999).contains(&t.year) {
                        let _ = write!(out, "{:04}", t.year);
                    } else {
                        let _ = write!(out, "{}", t.year);
                    }
                }
                Some('y') => {
                    let _ = write!(out, "{:02}", t.year.rem_euclid(100));
                }
                Some('m') => {
                    let _ = write!(out, "{:02}", t.month);
                }
                Some('d') => {
                    let _ = write!(out, "{:02}", t.day);
                }
                Some('H') => {
                    let _ = write!(out, "{:02}", t.hour);
                }
                Some('M') => {
                    let _ = write!(out, "{:02}", t.minute);
                }
                Some('S') => {
                    let _ = write!(out, "{:02}", t.second);
                }
                Some('I') => {
                    let h = if t.hour % 12 == 0 { 12 } else { t.hour % 12 };
                    let _ = write!(out, "{:02}", h);
                }
                Some('p') => out.push_str(if t.hour < 12 { "AM" } else { "PM" }),
                Some('j') => {
                    let _ = write!(out, "{:03}", t.yearday);
                }
                Some('a') => {
                    let nome = DIAS.get(t.weekday as usize).copied().unwrap_or("???");
                    out.push_str(&nome[..3]);
                }
                Some('A') => {
                    let nome = DIAS.get(t.weekday as usize).copied().unwrap_or("Unknown");
                    out.push_str(nome);
                }
                Some('b') => {
                    let nome = t
                        .month
                        .checked_sub(1)
                        .and_then(|idx| MESES.get(idx as usize))
                        .copied()
                        .unwrap_or("???");
                    out.push_str(&nome[..3]);
                }
                Some('B') => {
                    let nome = t
                        .month
                        .checked_sub(1)
                        .and_then(|idx| MESES.get(idx as usize))
                        .copied()
                        .unwrap_or("Unknown");
                    out.push_str(nome);
                }
                Some('Z') => out.push_str("UTC"),
                Some('F') => {
                    let _ = write!(out, "{:04}-{:02}-{:02}", t.year, t.month, t.day);
                }
                Some('T') => {
                    let _ = write!(out, "{:02}:{:02}:{:02}", t.hour, t.minute, t.second);
                }
                Some('%') => out.push('%'),
                Some(outro) => {
                    out.push('%');
                    out.push(outro);
                }
                None => out.push('%'),
            }
        }
        out
    }
}

// ============================================================================
// 10) Sistema de arquivos — os / os.path / pathlib / shutil + streaming/atomic
// ============================================================================
pub mod fs_utils {
    //! Operações de sistema de arquivos e leitura/escrita equivalentes a `os`, `os.path`,
    //! `shutil` e `open()` do Python.
    //!
    //! Na v2.0.0:
    //! - `glob` reescrito de forma iterativa ($O(N \cdot M)$, imune a backtracking exponencial).
    //! - `readlines` via `BufReader` e nova [`read_lines_lazy`] para arquivos grandes.
    //! - Leituras limitadas [`read_bytes_limited`] / [`read_text_limited`].
    //! - Escritas atômicas [`write_bytes_atomic`] / [`write_text_atomic`] via arquivo temporário
    //!   exclusivo no mesmo diretório + `sync_all` + `rename` (com ressalvas documentadas de SO).
    //! - [`copy_tree`] seguro contra loops de symlinks e [`metadata`] portável.

    use std::fs::{self, File, OpenOptions};
    use std::io::{self, BufRead, BufReader, Read, Write};
    #[cfg(unix)]
    use std::os::unix::fs::{symlink as unix_symlink, MetadataExt, PermissionsExt};
    use std::path::{Component, Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::UNIX_EPOCH;

    static ATOMIC_SEQ: AtomicU64 = AtomicU64::new(0);

    // ---------------- Leitura e escrita (bytes, texto, lazy e atômica) -------

    /// Lê todo o conteúdo de um arquivo de texto para uma `String`
    /// (equivalente a `open(caminho).read()` no Python).
    /// Para arquivos potencialmente grandes ou não confiáveis, use [`read_text_limited`] ou
    /// [`read_lines_lazy`].
    #[inline]
    pub fn read_text<P: AsRef<Path>>(caminho: P) -> io::Result<String> {
        fs::read_to_string(caminho)
    }

    /// Lê todo o arquivo como vetor de bytes (`Vec<u8>`).
    #[inline]
    pub fn read_bytes<P: AsRef<Path>>(caminho: P) -> io::Result<Vec<u8>> {
        fs::read(caminho)
    }

    /// Lê um arquivo de bytes impondo o limite máximo de `max_bytes`.
    /// Falha com `ErrorKind::InvalidData` se o arquivo exceder `max_bytes` antes de alocar além
    /// do teto permitido.
    pub fn read_bytes_limited<P: AsRef<Path>>(caminho: P, max_bytes: usize) -> io::Result<Vec<u8>> {
        let arq = File::open(caminho)?;
        if let Ok(meta) = arq.metadata() {
            if meta.len() > max_bytes as u64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "arquivo ({} bytes) excede o limite máximo de {} bytes",
                        meta.len(),
                        max_bytes
                    ),
                ));
            }
        }
        let cap = arq
            .metadata()
            .map(|m| (m.len() as usize).min(max_bytes))
            .unwrap_or(0);
        let mut buf = Vec::with_capacity(cap);
        let limite_u64 = (max_bytes as u64).saturating_add(1);
        arq.take(limite_u64).read_to_end(&mut buf)?;
        if buf.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("arquivo excedeu o limite máximo de {} bytes", max_bytes),
            ));
        }
        Ok(buf)
    }

    /// Lê um arquivo UTF-8 impondo o limite máximo de `max_bytes` bytes.
    pub fn read_text_limited<P: AsRef<Path>>(caminho: P, max_bytes: usize) -> io::Result<String> {
        let bytes = read_bytes_limited(caminho, max_bytes)?;
        String::from_utf8(bytes).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("arquivo não contém UTF-8 válido: {}", e),
            )
        })
    }

    /// Lê todas as linhas de um arquivo de texto via `BufReader` sem os terminadores `\n`/`\r\n`.
    pub fn readlines<P: AsRef<Path>>(caminho: P) -> io::Result<Vec<String>> {
        let arq = File::open(caminho)?;
        BufReader::new(arq).lines().collect()
    }

    /// Abre o arquivo e devolve um iterador **lazy** linha a linha via `BufReader`, permitindo
    /// processar arquivos de gigabytes com consumo de memória $O(1)$ por linha.
    pub fn read_lines_lazy<P: AsRef<Path>>(
        caminho: P,
    ) -> io::Result<io::Lines<BufReader<File>>> {
        let arq = File::open(caminho)?;
        Ok(BufReader::new(arq).lines())
    }

    /// Escreve (sobrescreve ou cria) texto em um arquivo
    /// (equivalente a `open(caminho, 'w').write(conteudo)`).
    #[inline]
    pub fn write_text<P: AsRef<Path>, C: AsRef<[u8]>>(caminho: P, conteudo: C) -> io::Result<()> {
        fs::write(caminho, conteudo)
    }

    /// Escreve (sobrescreve ou cria) bytes brutos em um arquivo.
    #[inline]
    pub fn write_bytes<P: AsRef<Path>, C: AsRef<[u8]>>(caminho: P, dados: C) -> io::Result<()> {
        fs::write(caminho, dados)
    }

    /// Acrescenta texto ao final de um arquivo, criando-o se não existir
    /// (equivalente a `open(caminho, 'a').write(conteudo)`).
    pub fn append_text<P: AsRef<Path>, C: AsRef<[u8]>>(caminho: P, conteudo: C) -> io::Result<()> {
        let mut arq = OpenOptions::new().create(true).append(true).open(caminho)?;
        arq.write_all(conteudo.as_ref())
    }

    /// Acrescenta bytes brutos ao final de um arquivo, criando-o se não existir.
    #[inline]
    pub fn append_bytes<P: AsRef<Path>, C: AsRef<[u8]>>(caminho: P, dados: C) -> io::Result<()> {
        append_text(caminho, dados)
    }

    /// Escreve `dados` criando um arquivo temporário exclusivo (`create_new(true)`) no mesmo
    /// diretório do destino, executando `flush` + `sync_all` e substituindo o alvo via `rename`.
    ///
    /// **Ressalvas do sistema de arquivos:**
    /// - Em sistemas POSIX, `rename` no mesmo sistema de arquivos é atômico para substituição da
    ///   entrada do diretório (o `fsync` do diretório pai não é feito pela `std` pura).
    /// - Em alguns sistemas de arquivos / Windows, bloqueios externos ou permissões podem impedir
    ///   a substituição de arquivos abertos por outro processo.
    pub fn write_bytes_atomic<P: AsRef<Path>, D: AsRef<[u8]>>(
        caminho: P,
        dados: D,
    ) -> io::Result<()> {
        let destino = caminho.as_ref();
        let dir = destino
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let nome_base = destino
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pyrsutils");

        let mut tmp_path = None;
        let mut tmp_file = None;
        for _ in 0..32 {
            let seq = ATOMIC_SEQ.fetch_add(1, Ordering::Relaxed);
            let candidato = dir.join(format!(".{}.tmp.{}.{}", nome_base, std::process::id(), seq));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidato)
            {
                Ok(f) => {
                    tmp_path = Some(candidato);
                    tmp_file = Some(f);
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }

        let tmp_path = tmp_path.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "não foi possível criar arquivo temporário exclusivo",
            )
        })?;
        let mut file = tmp_file.expect("arquivo temporário garantido");

        let res = (|| -> io::Result<()> {
            file.write_all(dados.as_ref())?;
            file.flush()?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp_path, destino)?;
            Ok(())
        })();

        if res.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        res
    }

    /// Variante textual de [`write_bytes_atomic`].
    #[inline]
    pub fn write_text_atomic<P: AsRef<Path>, S: AsRef<str>>(
        caminho: P,
        texto: S,
    ) -> io::Result<()> {
        write_bytes_atomic(caminho, texto.as_ref().as_bytes())
    }

    // ---------------- Consultas (os.path.exists/isfile/isdir/islink) -------
    #[inline]
    pub fn exists<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().exists()
    }
    #[inline]
    pub fn isfile<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().is_file()
    }
    #[inline]
    pub fn isdir<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().is_dir()
    }
    #[inline]
    pub fn islink<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().is_symlink()
    }

    // ---------------- Listar / criar / apagar / copiar (os / shutil) -------
    pub fn listdir<P: AsRef<Path>>(caminho: P) -> io::Result<Vec<String>> {
        fs::read_dir(caminho)?
            .map(|entrada| Ok(entrada?.file_name().to_string_lossy().into_owned()))
            .collect()
    }

    pub fn mkdir<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::create_dir(caminho)
    }

    pub fn makedirs<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::create_dir_all(caminho)
    }

    pub fn remove<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::remove_file(caminho)
    }

    pub fn rmdir<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::remove_dir(caminho)
    }

    pub fn rmtree<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::remove_dir_all(caminho)
    }

    pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(de: P, para: Q) -> io::Result<()> {
        fs::rename(de, para)
    }

    pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(de: P, para: Q) -> io::Result<u64> {
        fs::copy(de, para)
    }

    /// Copia recursivamente a árvore de diretórios `src` para `dst` (equivalente seguro a
    /// `shutil.copytree` ignorando links simbólicos para evitar loops infinitos). Retorna a
    /// quantidade de arquivos copiados.
    pub fn copy_tree<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<usize> {
        let src_root = src.as_ref();
        let dst_root = dst.as_ref();
        let meta_src = fs::symlink_metadata(src_root)?;
        if !meta_src.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "origem de copy_tree não é um diretório",
            ));
        }

        let mut copiados = 0usize;
        let mut pilha = vec![(src_root.to_path_buf(), dst_root.to_path_buf())];

        while let Some((cur_src, cur_dst)) = pilha.pop() {
            fs::create_dir_all(&cur_dst)?;
            for entry in fs::read_dir(&cur_src)? {
                let entry = entry?;
                let ft = entry.file_type()?;
                if ft.is_symlink() {
                    continue;
                }
                let src_item = entry.path();
                let dst_item = cur_dst.join(entry.file_name());
                if ft.is_dir() {
                    pilha.push((src_item, dst_item));
                } else if ft.is_file() {
                    fs::copy(&src_item, &dst_item)?;
                    copiados += 1;
                }
            }
        }
        Ok(copiados)
    }

    /// Cria um arquivo vazio se não existir (semelhante ao comando `touch`).
    pub fn touch<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(caminho)?;
        Ok(())
    }

    /// Retorna o tamanho em bytes de um arquivo regular (segue symlinks, via `fs::metadata`).
    #[inline]
    pub fn file_size<P: AsRef<Path>>(caminho: P) -> io::Result<u64> {
        Ok(fs::metadata(caminho)?.len())
    }

    // ---------------- Diretório atual (os.getcwd/chdir) --------------------
    pub fn getcwd() -> io::Result<String> {
        Ok(std::env::current_dir()?.to_string_lossy().into_owned())
    }

    pub fn chdir<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        std::env::set_current_dir(caminho)
    }

    // ---------------- Manipulação de caminhos (os.path) --------------------
    pub fn join_path(partes: &[&str]) -> String {
        let mut base = PathBuf::new();
        for p in partes {
            base.push(p);
        }
        base.to_string_lossy().into_owned()
    }

    pub fn basename<P: AsRef<Path>>(caminho: P) -> String {
        caminho
            .as_ref()
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn dirname<P: AsRef<Path>>(caminho: P) -> String {
        caminho
            .as_ref()
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn splitext<P: AsRef<Path>>(caminho: P) -> (String, String) {
        let p = caminho.as_ref();
        if p.file_name().is_none() {
            return (p.to_string_lossy().into_owned(), String::new());
        }
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = p
            .extension()
            .map(|s| format!(".{}", s.to_string_lossy()))
            .unwrap_or_default();
        let raiz = match p.parent().filter(|d| !d.as_os_str().is_empty()) {
            Some(dir) => dir.join(&stem).to_string_lossy().into_owned(),
            None => stem,
        };
        (raiz, ext)
    }

    pub fn abspath<P: AsRef<Path>>(caminho: P) -> io::Result<String> {
        let caminho = caminho.as_ref();
        let base = if caminho.is_absolute() {
            caminho.to_path_buf()
        } else {
            std::env::current_dir()?.join(caminho)
        };
        let mut normalizado = PathBuf::new();
        for componente in base.components() {
            match componente {
                Component::ParentDir => {
                    normalizado.pop();
                }
                Component::CurDir => {}
                outro => normalizado.push(outro.as_os_str()),
            }
        }
        Ok(normalizado.to_string_lossy().into_owned())
    }

    pub fn realpath<P: AsRef<Path>>(caminho: P) -> io::Result<String> {
        Ok(fs::canonicalize(caminho)?.to_string_lossy().into_owned())
    }

    /// Expande `~` inicial utilizando `HOME` (Unix) ou `USERPROFILE` (Windows).
    pub fn expanduser(caminho: &str) -> String {
        if let Some(resto) = caminho.strip_prefix('~') {
            if resto.is_empty() || resto.starts_with('/') || resto.starts_with('\\') {
                if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                    return format!("{}{}", home, resto);
                }
            }
        }
        caminho.to_string()
    }

    // ---------------- Percurso recursivo seguro (os.walk) ------------------
    pub type WalkItem = (String, Vec<String>, Vec<String>);

    pub struct Walk {
        pilha: Vec<PathBuf>,
    }

    pub fn walk<P: AsRef<Path>>(raiz: P) -> Walk {
        Walk {
            pilha: vec![raiz.as_ref().to_path_buf()],
        }
    }

    impl Iterator for Walk {
        type Item = io::Result<WalkItem>;

        fn next(&mut self) -> Option<Self::Item> {
            let dir = self.pilha.pop()?;
            let entradas = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(erro) => return Some(Err(erro)),
            };

            let mut subdirs = Vec::new();
            let mut arquivos = Vec::new();
            for entrada in entradas {
                let entrada = match entrada {
                    Ok(e) => e,
                    Err(erro) => return Some(Err(erro)),
                };
                let nome = entrada.file_name().to_string_lossy().into_owned();
                // Usa file_type() em vez de path().is_dir(): não segue links simbólicos
                // (impede loops infinitos) e evita chamadas stat() extras ao kernel.
                let eh_dir = entrada.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                if eh_dir {
                    self.pilha.push(entrada.path());
                    subdirs.push(nome);
                } else {
                    arquivos.push(nome);
                }
            }

            Some(Ok((
                dir.to_string_lossy().into_owned(),
                subdirs,
                arquivos,
            )))
        }
    }

    // ---------------- Glob iterativo O(N * M) (* e ?) ----------------------
    /// Correspondência de curingas `*` e `?` sem recursão e sem backtracking exponencial.
    fn glob_casa(padrao: &[char], nome: &[char]) -> bool {
        let (mut p, mut n) = (0usize, 0usize);
        let mut star_p: Option<usize> = None;
        let mut star_n: usize = 0;

        while n < nome.len() {
            if p < padrao.len() && (padrao[p] == '?' || padrao[p] == nome[n]) {
                p += 1;
                n += 1;
            } else if p < padrao.len() && padrao[p] == '*' {
                star_p = Some(p);
                star_n = n;
                p += 1;
            } else if let Some(sp) = star_p {
                p = sp + 1;
                star_n += 1;
                n = star_n;
            } else {
                return false;
            }
        }
        while p < padrao.len() && padrao[p] == '*' {
            p += 1;
        }
        p == padrao.len()
    }

    /// Busca entradas no diretório correspondente ao padrão com curingas `*` e `?` no último
    /// componente do caminho (não recursivo).
    pub fn glob(padrao: &str) -> io::Result<Vec<String>> {
        let caminho_padrao = Path::new(padrao);
        let dir = caminho_padrao.parent().filter(|p| !p.as_os_str().is_empty());
        let nome_padrao: Vec<char> = caminho_padrao
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
            .chars()
            .collect();

        let dir_busca = dir
            .map(|d| d.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let mut resultado = Vec::new();
        for entrada in fs::read_dir(&dir_busca)? {
            let entrada = entrada?;
            let nome = entrada.file_name().to_string_lossy().into_owned();
            let nome_chars: Vec<char> = nome.chars().collect();
            if glob_casa(&nome_padrao, &nome_chars) {
                resultado.push(match dir {
                    Some(d) => d.join(&nome).to_string_lossy().into_owned(),
                    None => nome,
                });
            }
        }
        Ok(resultado)
    }

    // ---------------- Metadados portáveis e Stat Unix ----------------------

    /// Metadados multiplataforma de um caminho (via `symlink_metadata`, sem seguir links).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileMetadata {
        pub size: u64,
        pub is_file: bool,
        pub is_dir: bool,
        pub is_symlink: bool,
        pub readonly: bool,
        pub modified_unix_secs: Option<i64>,
    }

    /// Obtém [`FileMetadata`] de forma portável (Unix e Windows) usando `symlink_metadata`.
    pub fn metadata<P: AsRef<Path>>(caminho: P) -> io::Result<FileMetadata> {
        let meta = fs::symlink_metadata(caminho)?;
        let modified_unix_secs = meta.modified().ok().map(|t| match t.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs().min(i64::MAX as u64) as i64,
            Err(e) => -(e.duration().as_secs().min(i64::MAX as u64) as i64),
        });
        Ok(FileMetadata {
            size: meta.len(),
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            is_symlink: meta.file_type().is_symlink(),
            readonly: meta.permissions().readonly(),
            modified_unix_secs,
        })
    }

    #[cfg(unix)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Stat {
        pub size: u64,
        pub mtime: i64,
        pub mode: u32,
    }

    /// Metadados detalhados POSIX (`#[cfg(unix)]` apenas).
    #[cfg(unix)]
    pub fn stat<P: AsRef<Path>>(caminho: P) -> io::Result<Stat> {
        let meta = fs::metadata(caminho)?;
        Ok(Stat {
            size: meta.size(),
            mtime: meta.mtime(),
            mode: meta.mode() & 0o7777,
        })
    }

    /// Altera bits de permissão POSIX (`#[cfg(unix)]` apenas).
    #[cfg(unix)]
    pub fn chmod<P: AsRef<Path>>(caminho: P, modo: u32) -> io::Result<()> {
        let mut perms = fs::metadata(&caminho)?.permissions();
        perms.set_mode(modo);
        fs::set_permissions(caminho, perms)
    }

    /// Cria um link simbólico POSIX (`#[cfg(unix)]` apenas).
    #[cfg(unix)]
    pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(alvo: P, link: Q) -> io::Result<()> {
        unix_symlink(alvo, link)
    }

    pub fn readlink<P: AsRef<Path>>(link: P) -> io::Result<String> {
        Ok(fs::read_link(link)?.to_string_lossy().into_owned())
    }
}

// ============================================================================
// 10B) io_utils — Leitores/Escritores bufferizados e streaming controlado
// ============================================================================
pub mod io_utils {
    //! Abstrações bufferizadas e funções de streaming para manipulação de arquivos e fluxos
    //! grandes sem carregar o conteúdo inteiro na memória.

    use std::fs::{File, OpenOptions};
    use std::io::{self, BufReader, BufWriter, Read, Write};
    use std::path::Path;

    /// Leitor bufferizado ergonômico sobre `BufReader<File>`.
    #[derive(Debug)]
    pub struct BufferedReader {
        inner: BufReader<File>,
    }

    impl BufferedReader {
        /// Abre `caminho` com o tamanho de buffer padrão da `std`.
        pub fn open<P: AsRef<Path>>(caminho: P) -> io::Result<Self> {
            Ok(BufferedReader {
                inner: BufReader::new(File::open(caminho)?),
            })
        }

        /// Abre `caminho` com buffer de capacidade explícita `cap`.
        pub fn with_capacity<P: AsRef<Path>>(caminho: P, cap: usize) -> io::Result<Self> {
            Ok(BufferedReader {
                inner: BufReader::with_capacity(cap.max(1), File::open(caminho)?),
            })
        }

        /// Lê uma linha removendo `\n`/`\r\n`. Retorna `Ok(None)` no EOF.
        pub fn read_line_trimmed(&mut self) -> io::Result<Option<String>> {
            use std::io::BufRead;
            let mut s = String::new();
            let n = self.inner.read_line(&mut s)?;
            if n == 0 {
                return Ok(None);
            }
            if s.ends_with('\n') {
                s.pop();
                if s.ends_with('\r') {
                    s.pop();
                }
            }
            Ok(Some(s))
        }

        /// Lê uma linha impondo limite máximo de `max_bytes` bytes (proteção contra DoS de memória).
        #[inline]
        pub fn read_line_limited(&mut self, max_bytes: usize) -> io::Result<Option<String>> {
            super::input_utils::read_line_from_limited(&mut self.inner, max_bytes)
        }

        /// Lê um bloco binário para `buf`, retornando o total de bytes lidos (`0` = EOF).
        #[inline]
        pub fn read_chunk(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.inner.read(buf)
        }

        /// Consome o leitor devolvendo um iterador lazy linha a linha.
        #[inline]
        pub fn lines(self) -> io::Lines<BufReader<File>> {
            use std::io::BufRead;
            self.inner.lines()
        }

        /// Copia todo o restante do arquivo para `dst` via streaming bufferizado.
        #[inline]
        pub fn copy_to<W: Write>(&mut self, dst: &mut W) -> io::Result<u64> {
            io::copy(&mut self.inner, dst)
        }
    }

    /// Escritor bufferizado ergonômico sobre `BufWriter<File>`.
    #[derive(Debug)]
    pub struct BufferedWriter {
        inner: BufWriter<File>,
    }

    impl BufferedWriter {
        /// Cria ou sobrescreve `caminho` com `BufWriter`.
        pub fn create<P: AsRef<Path>>(caminho: P) -> io::Result<Self> {
            Ok(BufferedWriter {
                inner: BufWriter::new(File::create(caminho)?),
            })
        }

        /// Abre `caminho` em modo `append` (criando se não existir) com `BufWriter`.
        pub fn append<P: AsRef<Path>>(caminho: P) -> io::Result<Self> {
            let arq = OpenOptions::new().create(true).append(true).open(caminho)?;
            Ok(BufferedWriter {
                inner: BufWriter::new(arq),
            })
        }

        /// Cria ou sobrescreve `caminho` com buffer de capacidade `cap`.
        pub fn with_capacity<P: AsRef<Path>>(caminho: P, cap: usize) -> io::Result<Self> {
            Ok(BufferedWriter {
                inner: BufWriter::with_capacity(cap.max(1), File::create(caminho)?),
            })
        }

        /// Escreve bytes no buffer.
        #[inline]
        pub fn write_bytes(&mut self, data: &[u8]) -> io::Result<()> {
            self.inner.write_all(data)
        }

        /// Escreve texto UTF-8 no buffer.
        #[inline]
        pub fn write_str(&mut self, s: &str) -> io::Result<()> {
            self.inner.write_all(s.as_bytes())
        }

        /// Escreve texto seguido de `\n`.
        pub fn write_line(&mut self, s: &str) -> io::Result<()> {
            self.inner.write_all(s.as_bytes())?;
            self.inner.write_all(b"\n")
        }

        /// Descarrega o buffer para o arquivo subjacente.
        #[inline]
        pub fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }

    /// Copia bytes de `src` para `dst` respeitando o teto `max_bytes`.
    /// Falha com `ErrorKind::InvalidData` se `src` tentar fornecer mais do que `max_bytes` bytes.
    pub fn copy_stream_limited<R: Read, W: Write>(
        src: &mut R,
        dst: &mut W,
        max_bytes: u64,
    ) -> io::Result<u64> {
        let mut buf = [0u8; 8192];
        let mut total = 0u64;
        loop {
            let restante = max_bytes.saturating_sub(total);
            let a_ler = if restante == 0 {
                1
            } else {
                (restante as usize).min(buf.len())
            };
            match src.read(&mut buf[..a_ler]) {
                Ok(0) => return Ok(total),
                Ok(n) => {
                    if total == max_bytes {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("fluxo excedeu o limite máximo de {} bytes", max_bytes),
                        ));
                    }
                    dst.write_all(&buf[..n])?;
                    total += n as u64;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }

    /// Lê até `max_bytes` bytes de um leitor genérico `R: Read`. Retorna erro `InvalidData` se o
    /// fluxo contiver mais que `max_bytes` bytes.
    pub fn read_to_vec_limited<R: Read>(src: &mut R, max_bytes: usize) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        let cap_u64 = (max_bytes as u64).saturating_add(1);
        src.take(cap_u64).read_to_end(&mut out)?;
        if out.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("stream excedeu o limite máximo de {} bytes", max_bytes),
            ));
        }
        Ok(out)
    }

    /// Lê até `max_bytes` bytes UTF-8 de um leitor genérico `R: Read` como `String`.
    pub fn read_to_string_limited<R: Read>(src: &mut R, max_bytes: usize) -> io::Result<String> {
        let bytes = read_to_vec_limited(src, max_bytes)?;
        String::from_utf8(bytes).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("UTF-8 inválido no stream: {}", e),
            )
        })
    }

    /// Percorre todas as linhas de um arquivo reutilizando um único buffer `String` (zero
    /// alocação por linha), passando `(numero_linha_0_based, linha_sem_crlf)` para `f`.
    /// Retorna o total de linhas processadas.
    pub fn for_each_line<P, F>(caminho: P, mut f: F) -> io::Result<usize>
    where
        P: AsRef<Path>,
        F: FnMut(usize, &str) -> io::Result<()>,
    {
        use std::io::BufRead;
        let mut reader = BufReader::new(File::open(caminho)?);
        let mut buf = String::new();
        let mut count = 0usize;
        loop {
            buf.clear();
            let n = reader.read_line(&mut buf)?;
            if n == 0 {
                break;
            }
            if buf.ends_with('\n') {
                buf.pop();
                if buf.ends_with('\r') {
                    buf.pop();
                }
            }
            f(count, &buf)?;
            count += 1;
        }
        Ok(count)
    }
}

// ============================================================================
// 10C) env_utils — Camada ergonômica sobre std::env
// ============================================================================
pub mod env_utils {
    //! Wrappers ergonômicos sobre `std::env`, preservando caminhos e variáveis não-UTF8 via
    //! `OsString` / `PathBuf` quando necessário.
    //!
    //! **Transparência técnica e segurança:**
    //! - [`set_current_dir`] altera o diretório de trabalho de **todo o processo** (efeito global
    //!   visível a todas as threads).
    //! - Funções de mutação de variáveis de ambiente (`set_var` / `remove_var`) **não são
    //!   expostas** como seguras nesta biblioteca porque em ambientes POSIX multithread elas causam
    //!   condições de corrida inseguras (`unsafe` a partir da Rust Edition 2024).

    use std::collections::HashMap;
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::str::FromStr;

    /// Argumentos da linha de comando como `Vec<String>` (ignora argumentos não-UTF8 usando
    /// conversão lossy de `args_os`, evitando panic da `std::env::args()`).
    pub fn args() -> Vec<String> {
        std::env::args_os()
            .map(|s| s.to_string_lossy().into_owned())
            .collect()
    }

    /// Iterador lazy sobre argumentos UTF-8 (equivalente a `std::env::args()`).
    #[inline]
    pub fn args_iter() -> std::env::Args {
        std::env::args()
    }

    /// Argumentos brutos da linha de comando preservando bytes não-UTF8 (`Vec<OsString>`).
    #[inline]
    pub fn args_os() -> Vec<OsString> {
        std::env::args_os().collect()
    }

    /// Retorna o `index`-ésimo argumento de linha de comando (lossy se não-UTF8), ou `None`.
    pub fn arg(index: usize) -> Option<String> {
        std::env::args_os()
            .nth(index)
            .map(|s| s.to_string_lossy().into_owned())
    }

    /// Lê a variável de ambiente `key` como `String`. Retorna `None` se ausente ou não-UTF8.
    /// Para distinguir ausência de UTF-8 inválido, use [`var_strict`] ou [`var_os`].
    #[inline]
    pub fn var(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    /// Lê a variável de ambiente `key` preservando o erro exato (`NotPresent` / `NotUnicode`).
    #[inline]
    pub fn var_strict(key: &str) -> Result<String, std::env::VarError> {
        std::env::var(key)
    }

    /// Lê `key` como `String` ou devolve `default` se ausente ou não-UTF8.
    #[inline]
    pub fn var_or(key: &str, default: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| default.to_string())
    }

    /// Lê e converte uma variável de ambiente para `T: FromStr`.
    /// Retorna `None` se a variável não existir (ou não for UTF-8), ou `Some(Result)` do parse.
    #[inline]
    pub fn var_parse<T: FromStr>(key: &str) -> Option<Result<T, T::Err>> {
        std::env::var(key).ok().map(|s| s.parse::<T>())
    }

    /// Lê uma variável de ambiente como `OsString` (funciona mesmo para valores não-UTF8).
    #[inline]
    pub fn var_os<K: AsRef<OsStr>>(key: K) -> Option<OsString> {
        std::env::var_os(key)
    }

    /// Retorna `true` se a variável de ambiente `key` estiver definida (independente de UTF-8).
    #[inline]
    pub fn has_var<K: AsRef<OsStr>>(key: K) -> bool {
        std::env::var_os(key).is_some()
    }

    /// Mapa das variáveis de ambiente UTF-8 válidas (ignora entradas não-UTF8 como faz `env::vars`).
    #[inline]
    pub fn vars() -> HashMap<String, String> {
        std::env::vars().collect()
    }

    /// Mapa completo das variáveis de ambiente brutas (`OsString`).
    #[inline]
    pub fn vars_os() -> HashMap<OsString, OsString> {
        std::env::vars_os().collect()
    }

    /// Diretório de trabalho atual do processo (`std::env::current_dir`).
    #[inline]
    pub fn current_dir() -> io::Result<PathBuf> {
        std::env::current_dir()
    }

    /// Caminho do executável atual (`std::env::current_exe`).
    #[inline]
    pub fn current_exe() -> io::Result<PathBuf> {
        std::env::current_exe()
    }

    /// Diretório temporário padrão do sistema operacional (`std::env::temp_dir`).
    #[inline]
    pub fn temp_dir() -> PathBuf {
        std::env::temp_dir()
    }

    /// Altera o diretório de trabalho atual do processo.
    ///
    /// **Atenção:** o efeito é global para todas as threads do processo.
    #[inline]
    pub fn set_current_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
        std::env::set_current_dir(path)
    }

    /// Nome do sistema operacional em tempo de compilação (ex.: `"linux"`, `"windows"`, `"macos"`).
    #[inline]
    pub const fn os_name() -> &'static str {
        std::env::consts::OS
    }

    /// Arquitetura da CPU alvo em tempo de compilação (ex.: `"x86_64"`, `"aarch64"`).
    #[inline]
    pub const fn arch() -> &'static str {
        std::env::consts::ARCH
    }

    /// Família do sistema operacional (ex.: `"unix"`, `"windows"`).
    #[inline]
    pub const fn os_family() -> &'static str {
        std::env::consts::FAMILY
    }

    /// Divide uma variável estilo `PATH` respeitando o separador do sistema (`:` no Unix, `;` no Windows).
    pub fn split_paths_env<K: AsRef<OsStr>>(key: K) -> Vec<PathBuf> {
        match std::env::var_os(key) {
            Some(val) => std::env::split_paths(&val).collect(),
            None => Vec::new(),
        }
    }

    /// Obtém o diretório `HOME` (Unix) ou `USERPROFILE` (Windows) a partir das variáveis de ambiente.
    pub fn home_dir() -> Option<PathBuf> {
        #[cfg(unix)]
        {
            std::env::var_os("HOME").map(PathBuf::from)
        }
        #[cfg(windows)]
        {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .or_else(|| {
                    let drive = std::env::var_os("HOMEDRIVE")?;
                    let path = std::env::var_os("HOMEPATH")?;
                    let mut p = OsString::from(drive);
                    p.push(path);
                    Some(PathBuf::from(p))
                })
        }
        #[cfg(not(any(unix, windows)))]
        {
            std::env::var_os("HOME").map(PathBuf::from)
        }
    }

    /// Estimativa de paralelismo de hardware disponível (com fallback seguro para `1`).
    #[inline]
    pub fn available_parallelism() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }
}

// ============================================================================
// 10D) process_utils — Execução segura sobre std::process (sem shell injection)
// ============================================================================
pub mod process_utils {
    //! Wrappers seguros e ergonômicos sobre `std::process::Command` e `Child`.
    //!
    //! **Segurança:**
    //! - Utiliza sempre `Command::new(programa).args(...)` (passagem direta de argumentos ao
    //!   kernel via `execve`/`CreateProcess`), **nunca** invocando `sh -c` ou `cmd /C` com strings
    //!   concatenadas, prevenindo vulnerabilidades de *command injection*.
    //! - **Verificação de status:** a conclusão de [`run`] ou `Command::output()` com `Ok(...)`
    //!   significa apenas que o processo iniciou e terminou, **não** que retornou código 0.
    //!   Verifique `output.success()` ou utilize [`run_checked`] / [`ProcessOutput::check`].

    use std::ffi::OsStr;
    use std::fmt;
    use std::io::{self, Write};
    use std::path::Path;
    use std::process::{
        Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio,
    };
    use std::time::{Duration, Instant};

    /// Erro explícito de execução ou validação de subprocessos.
    #[derive(Debug)]
    pub enum ProcessError {
        /// Falha de I/O ao iniciar ou comunicar com o subprocesso.
        Io(io::Error),
        /// O processo executou, mas terminou com código diferente de zero (ou por sinal).
        NonZeroExit {
            program: String,
            code: Option<i32>,
            stderr: String,
        },
        /// Pipeline vazio ou inválido.
        InvalidPipeline,
        /// Timeout inválido (negativo, `NaN` ou infinito).
        InvalidTimeout,
    }

    impl fmt::Display for ProcessError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ProcessError::Io(e) => write!(f, "IOError: falha no subprocesso ({})", e),
                ProcessError::NonZeroExit {
                    program,
                    code,
                    stderr,
                } => write!(
                    f,
                    "SubprocessError: '{}' encerrou com status {:?}: {}",
                    program,
                    code,
                    stderr.trim()
                ),
                ProcessError::InvalidPipeline => {
                    write!(f, "ValueError: pipeline de processos não pode ser vazio")
                }
                ProcessError::InvalidTimeout => {
                    write!(f, "ValueError: timeout inválido para processo")
                }
            }
        }
    }

    impl std::error::Error for ProcessError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ProcessError::Io(e) => Some(e),
                _ => None,
            }
        }
    }

    impl From<io::Error> for ProcessError {
        fn from(e: io::Error) -> Self {
            ProcessError::Io(e)
        }
    }

    /// Saída capturada de um subprocesso finalizado.
    #[derive(Debug, Clone)]
    pub struct ProcessOutput {
        pub status: ExitStatus,
        pub exit_code: Option<i32>,
        pub stdout: Vec<u8>,
        pub stderr: Vec<u8>,
    }

    impl ProcessOutput {
        fn from_std(out: std::process::Output) -> Self {
            ProcessOutput {
                exit_code: out.status.code(),
                status: out.status,
                stdout: out.stdout,
                stderr: out.stderr,
            }
        }

        /// Retorna `true` somente se o processo encerrou com sucesso (código 0 no POSIX/Windows).
        #[inline]
        pub fn success(&self) -> bool {
            self.status.success()
        }

        /// Decodifica `stdout` como UTF-8 *lossy* (substitui sequências inválidas por U+FFFD).
        #[inline]
        pub fn stdout_text(&self) -> String {
            String::from_utf8_lossy(&self.stdout).into_owned()
        }

        /// Decodifica `stderr` como UTF-8 *lossy*.
        #[inline]
        pub fn stderr_text(&self) -> String {
            String::from_utf8_lossy(&self.stderr).into_owned()
        }

        /// Decodifica `stdout` estritamente como UTF-8.
        #[inline]
        pub fn stdout_utf8(&self) -> Result<String, std::string::FromUtf8Error> {
            String::from_utf8(self.stdout.clone())
        }

        /// Decodifica `stderr` estritamente como UTF-8.
        #[inline]
        pub fn stderr_utf8(&self) -> Result<String, std::string::FromUtf8Error> {
            String::from_utf8(self.stderr.clone())
        }

        /// Valida se `self.success() == true`; caso contrário devolve [`ProcessError::NonZeroExit`].
        pub fn check(self, program: &str) -> Result<Self, ProcessError> {
            if self.status.success() {
                Ok(self)
            } else {
                Err(ProcessError::NonZeroExit {
                    program: program.to_string(),
                    code: self.exit_code,
                    stderr: self.stderr_text(),
                })
            }
        }
    }

    /// Wrapper gerenciado sobre [`std::process::Child`].
    #[derive(Debug)]
    pub struct ManagedChild {
        inner: Child,
    }

    impl ManagedChild {
        /// PID do subprocesso.
        #[inline]
        pub fn id(&self) -> u32 {
            self.inner.id()
        }

        /// Aguarda bloqueando até o processo encerrar.
        #[inline]
        pub fn wait(&mut self) -> io::Result<ExitStatus> {
            self.inner.wait()
        }

        /// Verifica sem bloquear se o processo já terminou.
        #[inline]
        pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.inner.try_wait()
        }

        /// Aguarda até `timeout_secs` segundos pelo término do processo via polling adaptativo.
        /// Retorna `Ok(None)` se o timeout expirar antes do término (o processo continua vivo).
        pub fn wait_timeout(&mut self, timeout_secs: f64) -> Result<Option<ExitStatus>, ProcessError> {
            let dur = Duration::try_from_secs_f64(timeout_secs)
                .map_err(|_| ProcessError::InvalidTimeout)?;
            let limite = Instant::now() + dur;
            let mut passo = Duration::from_millis(1);
            loop {
                if let Some(status) = self.inner.try_wait()? {
                    return Ok(Some(status));
                }
                let agora = Instant::now();
                if agora >= limite {
                    return Ok(None);
                }
                let restante = limite - agora;
                std::thread::sleep(passo.min(restante));
                passo = (passo * 2).min(Duration::from_millis(10));
            }
        }

        /// Consome o manipulador e aguarda lendo todo `stdout` e `stderr`.
        pub fn wait_with_output(self) -> io::Result<ProcessOutput> {
            Ok(ProcessOutput::from_std(self.inner.wait_with_output()?))
        }

        /// Envia sinal de término imediato ao processo filho.
        #[inline]
        pub fn kill(&mut self) -> io::Result<()> {
            self.inner.kill()
        }

        /// Mata o processo filho e executa `wait()` para evitar processos zumbis.
        pub fn kill_and_wait(&mut self) -> io::Result<ExitStatus> {
            let _ = self.inner.kill();
            self.inner.wait()
        }

        /// Escreve bytes no `stdin` do processo filho (se foi configurado com pipe).
        pub fn write_stdin(&mut self, data: &[u8]) -> io::Result<()> {
            match self.inner.stdin.as_mut() {
                Some(stdin) => {
                    stdin.write_all(data)?;
                    stdin.flush()
                }
                None => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "stdin do subprocesso não está disponível",
                )),
            }
        }

        /// Fecha o `stdin` do subprocesso enviando EOF.
        #[inline]
        pub fn close_stdin(&mut self) {
            let _ = self.inner.stdin.take();
        }

        #[inline]
        pub fn take_stdin(&mut self) -> Option<ChildStdin> {
            self.inner.stdin.take()
        }
        #[inline]
        pub fn take_stdout(&mut self) -> Option<ChildStdout> {
            self.inner.stdout.take()
        }
        #[inline]
        pub fn take_stderr(&mut self) -> Option<ChildStderr> {
            self.inner.stderr.take()
        }
    }

    /// Construtor fluente de subprocessos seguro contra *command injection*.
    #[derive(Debug)]
    pub struct ProcessBuilder {
        program: String,
        cmd: Command,
        stdin_bytes: Option<Vec<u8>>,
    }

    impl ProcessBuilder {
        /// Inicia a configuração para executar `program` diretamente (sem shell).
        pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
            let s = program.as_ref();
            ProcessBuilder {
                program: s.to_string_lossy().into_owned(),
                cmd: Command::new(s),
                stdin_bytes: None,
            }
        }

        /// Adiciona um único argumento literal.
        pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
            self.cmd.arg(arg);
            self
        }

        /// Adiciona múltiplos argumentos literais sem interpretação por shell.
        pub fn args<I, S>(mut self, args: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            self.cmd.args(args);
            self
        }

        /// Define o diretório de trabalho específico apenas para este subprocesso.
        pub fn cwd<P: AsRef<Path>>(mut self, dir: P) -> Self {
            self.cmd.current_dir(dir);
            self
        }

        /// Define uma variável de ambiente apenas no subprocesso.
        pub fn env<K: AsRef<OsStr>, V: AsRef<OsStr>>(mut self, k: K, v: V) -> Self {
            self.cmd.env(k, v);
            self
        }

        /// Limpa todas as variáveis de ambiente herdadas antes de definir outras com `.env(...)`.
        pub fn env_clear(mut self) -> Self {
            self.cmd.env_clear();
            self
        }

        /// Configura bytes a serem entregues via `stdin` quando [`run`](Self::run) for chamado.
        pub fn stdin_data<D: AsRef<[u8]>>(mut self, data: D) -> Self {
            self.stdin_bytes = Some(data.as_ref().to_vec());
            self
        }

        /// Executa o subprocesso capturando `stdout` e `stderr`.
        ///
        /// **Importante:** um retorno `Ok(out)` apenas garante que o processo executou; verifique
        /// `out.success()` ou utilize [`run_checked`](Self::run_checked).
        pub fn run(mut self) -> io::Result<ProcessOutput> {
            match self.stdin_bytes.take() {
                None => Ok(ProcessOutput::from_std(self.cmd.output()?)),
                Some(bytes) => {
                    self.cmd
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped());
                    let mut child = self.cmd.spawn()?;
                    let mut stdin_pipe = child.stdin.take();
                    // Escreve stdin em thread escopada para evitar deadlock caso stdout/stderr
                    // saturem o pipe enquanto o pai ainda escreve no stdin.
                    std::thread::scope(|s| {
                        let writer = s.spawn(move || -> io::Result<()> {
                            if let Some(mut pin) = stdin_pipe.take() {
                                let _ = pin.write_all(&bytes);
                                let _ = pin.flush();
                            }
                            Ok(())
                        });
                        let res = child.wait_with_output();
                        let _ = writer.join();
                        res.map(ProcessOutput::from_std)
                    })
                }
            }
        }

        /// Executa o subprocesso e falha com [`ProcessError::NonZeroExit`] se o código de saída
        /// for diferente de zero.
        pub fn run_checked(self) -> Result<ProcessOutput, ProcessError> {
            let prog = self.program.clone();
            let out = self.run()?;
            out.check(&prog)
        }

        /// Inicia o subprocesso em background retornando [`ManagedChild`].
        pub fn spawn(
            mut self,
            pipe_stdin: bool,
            pipe_stdout: bool,
            pipe_stderr: bool,
        ) -> io::Result<ManagedChild> {
            self.cmd.stdin(if pipe_stdin {
                Stdio::piped()
            } else {
                Stdio::inherit()
            });
            self.cmd.stdout(if pipe_stdout {
                Stdio::piped()
            } else {
                Stdio::inherit()
            });
            self.cmd.stderr(if pipe_stderr {
                Stdio::piped()
            } else {
                Stdio::inherit()
            });
            Ok(ManagedChild {
                inner: self.cmd.spawn()?,
            })
        }
    }

    /// Executa `program` com `args` e devolve [`ProcessOutput`].
    /// Lembre-se de validar `out.success()` ou usar [`run_checked`].
    pub fn run<S, I, A>(program: S, args: I) -> io::Result<ProcessOutput>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        ProcessBuilder::new(program).args(args).run()
    }

    /// Executa `program` com `args` exigindo código de saída 0.
    pub fn run_checked<S, I, A>(program: S, args: I) -> Result<ProcessOutput, ProcessError>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        ProcessBuilder::new(program).args(args).run_checked()
    }

    /// Executa `program` dentro de um diretório de trabalho específico.
    pub fn run_in_dir<S, I, A, P>(program: S, args: I, dir: P) -> io::Result<ProcessOutput>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        ProcessBuilder::new(program).args(args).cwd(dir).run()
    }

    /// Executa `program` enviando `stdin_data` para sua entrada padrão.
    pub fn run_with_stdin<S, I, A, D>(
        program: S,
        args: I,
        stdin_data: D,
    ) -> io::Result<ProcessOutput>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
        D: AsRef<[u8]>,
    {
        ProcessBuilder::new(program)
            .args(args)
            .stdin_data(stdin_data)
            .run()
    }

    /// Executa um pipeline sequencial de estágios (`[(programa, &[args])]`), conectando o
    /// `stdout` de cada estágio ao `stdin` do próximo sem invocar shell.
    pub fn run_pipeline(stages: &[(&str, &[&str])]) -> Result<ProcessOutput, ProcessError> {
        if stages.is_empty() {
            return Err(ProcessError::InvalidPipeline);
        }
        let n = stages.len();
        let mut children: Vec<Child> = Vec::with_capacity(n);
        let mut prev_stdout: Option<ChildStdout> = None;

        for (idx, &(prog, args)) in stages.iter().enumerate() {
            let mut cmd = Command::new(prog);
            cmd.args(args);
            if let Some(out) = prev_stdout.take() {
                cmd.stdin(Stdio::from(out));
            }
            if idx + 1 < n {
                cmd.stdout(Stdio::piped());
            } else {
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
            }

            match cmd.spawn() {
                Ok(mut child) => {
                    if idx + 1 < n {
                        prev_stdout = child.stdout.take();
                    }
                    children.push(child);
                }
                Err(e) => {
                    for mut c in children {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    return Err(ProcessError::Io(e));
                }
            }
        }

        let ultimo = children.pop().expect("pipeline não vazio garantido");
        let res_ultimo = ultimo.wait_with_output();
        // Aguarda os processos anteriores para não deixar zumbis
        for mut c in children {
            let _ = c.wait();
        }
        Ok(ProcessOutput::from_std(res_ultimo?))
    }
}

// ============================================================================
// 10E) terminal_utils — CLI ergonômica, TTY, ANSI, Prompts e Barra de Progresso
// ============================================================================
pub mod terminal_utils {
    //! Utilitários para aplicações de linha de comando (CLI): detecção de TTY, estilos/cores
    //! ANSI configuráveis (respeitando redirecionamento e `NO_COLOR`), prompts yes/no, menus e
    //! barra de progresso.
    //!
    //! **Limitações da `std`:** modos de terminal bruto (*raw mode*), leitura de tecla única sem
    //! Enter e ocultação portátil de senha não são fornecidos pela `std` sem syscalls/FFI de OS.

    use std::fmt::Write as _;
    use std::io::{self, BufRead, IsTerminal, Write};

    #[inline]
    pub fn is_stdin_tty() -> bool {
        io::stdin().is_terminal()
    }

    #[inline]
    pub fn is_stdout_tty() -> bool {
        io::stdout().is_terminal()
    }

    #[inline]
    pub fn is_stderr_tty() -> bool {
        io::stderr().is_terminal()
    }

    /// Política de emissão de códigos de cor/estilo ANSI.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ColorMode {
        /// Emite cores somente quando o fluxo de saída é um TTY e `NO_COLOR` não está definida.
        Auto,
        /// Emite códigos ANSI incondicionalmente.
        Always,
        /// Nunca emite códigos ANSI.
        Never,
    }

    /// Cores ANSI padrão de 4 bits (16 cores).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Color {
        Black,
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        White,
        BrightBlack,
        BrightRed,
        BrightGreen,
        BrightYellow,
        BrightBlue,
        BrightMagenta,
        BrightCyan,
        BrightWhite,
    }

    impl Color {
        const fn fg_code(self) -> u8 {
            match self {
                Color::Black => 30,
                Color::Red => 31,
                Color::Green => 32,
                Color::Yellow => 33,
                Color::Blue => 34,
                Color::Magenta => 35,
                Color::Cyan => 36,
                Color::White => 37,
                Color::BrightBlack => 90,
                Color::BrightRed => 91,
                Color::BrightGreen => 92,
                Color::BrightYellow => 93,
                Color::BrightBlue => 94,
                Color::BrightMagenta => 95,
                Color::BrightCyan => 96,
                Color::BrightWhite => 97,
            }
        }

        const fn bg_code(self) -> u8 {
            match self {
                Color::Black => 40,
                Color::Red => 41,
                Color::Green => 42,
                Color::Yellow => 43,
                Color::Blue => 44,
                Color::Magenta => 45,
                Color::Cyan => 46,
                Color::White => 47,
                Color::BrightBlack => 100,
                Color::BrightRed => 101,
                Color::BrightGreen => 102,
                Color::BrightYellow => 103,
                Color::BrightBlue => 104,
                Color::BrightMagenta => 105,
                Color::BrightCyan => 106,
                Color::BrightWhite => 107,
            }
        }
    }

    /// Descrição de estilo visual ANSI para texto de terminal.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Style {
        pub fg: Option<Color>,
        pub bg: Option<Color>,
        pub bold: bool,
        pub dim: bool,
        pub underline: bool,
    }

    impl Style {
        #[inline]
        pub const fn new() -> Self {
            Style {
                fg: None,
                bg: None,
                bold: false,
                dim: false,
                underline: false,
            }
        }

        #[inline]
        pub const fn fg(mut self, color: Color) -> Self {
            self.fg = Some(color);
            self
        }

        #[inline]
        pub const fn bg(mut self, color: Color) -> Self {
            self.bg = Some(color);
            self
        }

        #[inline]
        pub const fn bold(mut self) -> Self {
            self.bold = true;
            self
        }

        #[inline]
        pub const fn dim(mut self) -> Self {
            self.dim = true;
            self
        }

        #[inline]
        pub const fn underline(mut self) -> Self {
            self.underline = true;
            self
        }
    }

    fn no_color_env() -> bool {
        std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
    }

    /// Indica se cores devem ser usadas em `stdout` conforme `mode`, `NO_COLOR` e `is_stdout_tty()`.
    pub fn colors_enabled(mode: ColorMode) -> bool {
        match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => is_stdout_tty() && !no_color_env(),
        }
    }

    /// Indica se cores devem ser usadas em `stderr` conforme `mode`, `NO_COLOR` e `is_stderr_tty()`.
    pub fn colors_enabled_stderr(mode: ColorMode) -> bool {
        match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => is_stderr_tty() && !no_color_env(),
        }
    }

    fn apply_style_raw(text: &str, style: Style) -> String {
        let mut codes: Vec<String> = Vec::new();
        if style.bold {
            codes.push("1".to_string());
        }
        if style.dim {
            codes.push("2".to_string());
        }
        if style.underline {
            codes.push("4".to_string());
        }
        if let Some(fg) = style.fg {
            codes.push(fg.fg_code().to_string());
        }
        if let Some(bg) = style.bg {
            codes.push(bg.bg_code().to_string());
        }
        if codes.is_empty() {
            return text.to_string();
        }
        format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
    }

    /// Aplica estilo ANSI a `text` segundo `mode` (para `stdout`).
    pub fn style_text(text: &str, style: Style, mode: ColorMode) -> String {
        if !colors_enabled(mode) {
            return text.to_string();
        }
        apply_style_raw(text, style)
    }

    /// Aplica estilo ANSI a `text` verificando TTY de `stderr` quando `mode == ColorMode::Auto`.
    pub fn style_text_stderr(text: &str, style: Style, mode: ColorMode) -> String {
        if !colors_enabled_stderr(mode) {
            return text.to_string();
        }
        apply_style_raw(text, style)
    }

    /// Colore `text` com `color` respeitando `mode` para `stdout`.
    #[inline]
    pub fn colorize(text: &str, color: Color, mode: ColorMode) -> String {
        style_text(text, Style::new().fg(color), mode)
    }

    /// Colore `text` com sequências ANSI incondicionalmente (útil para testes e logs ANSI).
    #[inline]
    pub fn colorize_always(text: &str, color: Color) -> String {
        style_text(text, Style::new().fg(color), ColorMode::Always)
    }

    /// Remove sequências de escape ANSI CSI (`ESC [ ... <letra>`) de uma string.
    pub fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7E).contains(&b) {
                        break;
                    }
                }
            } else if let Some(ch) = s[i..].chars().next() {
                out.push(ch);
                i += ch.len_utf8();
            } else {
                break;
            }
        }
        out
    }

    /// Imprime mensagem informativa `[INFO]` em `stdout`.
    pub fn print_info(msg: &str) {
        let tag = style_text("[INFO]", Style::new().fg(Color::Cyan).bold(), ColorMode::Auto);
        println!("{} {}", tag, msg);
    }

    /// Imprime mensagem de sucesso `[OK]` em `stdout`.
    pub fn print_ok(msg: &str) {
        let tag = style_text("[OK]", Style::new().fg(Color::Green).bold(), ColorMode::Auto);
        println!("{} {}", tag, msg);
    }

    /// Imprime aviso `[WARN]` em `stderr`.
    pub fn print_warn(msg: &str) {
        let tag = style_text_stderr(
            "[WARN]",
            Style::new().fg(Color::Yellow).bold(),
            ColorMode::Auto,
        );
        eprintln!("{} {}", tag, msg);
    }

    /// Imprime erro `[ERROR]` em `stderr`.
    pub fn print_error(msg: &str) {
        let tag = style_text_stderr(
            "[ERROR]",
            Style::new().fg(Color::Red).bold(),
            ColorMode::Auto,
        );
        eprintln!("{} {}", tag, msg);
    }

    /// Solicita confirmação yes/no usando leitor e escritor arbitrários (testável sem TTY real).
    pub fn confirm_with<R: BufRead, W: Write>(
        reader: &mut R,
        writer: &mut W,
        prompt: &str,
        default: Option<bool>,
    ) -> io::Result<bool> {
        let sufixo = match default {
            Some(true) => " [S/n]: ",
            Some(false) => " [s/N]: ",
            None => " [s/n]: ",
        };

        loop {
            write!(writer, "{}{}", prompt, sufixo)?;
            writer.flush()?;

            let mut linha = String::new();
            let n = reader.read_line(&mut linha)?;
            if n == 0 {
                return match default {
                    Some(d) => Ok(d),
                    None => Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "EOF ao aguardar confirmação",
                    )),
                };
            }
            let resp = linha.trim().to_ascii_lowercase();
            if resp.is_empty() {
                if let Some(d) = default {
                    return Ok(d);
                }
                continue;
            }
            match resp.as_str() {
                "s" | "sim" | "y" | "yes" => return Ok(true),
                "n" | "nao" | "não" | "no" => return Ok(false),
                _ => {
                    writeln!(writer, "Responda com 's' ou 'n'.")?;
                }
            }
        }
    }

    /// Solicita confirmação interativa yes/no em `stdin`/`stdout`.
    #[inline]
    pub fn confirm(prompt: &str, default: Option<bool>) -> io::Result<bool> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut r = stdin.lock();
        let mut w = stdout.lock();
        confirm_with(&mut r, &mut w, prompt, default)
    }

    /// Exibe menu numerado (`1..=N`) e retorna o índice 0-based selecionado.
    pub fn select_menu_with<R: BufRead, W: Write>(
        reader: &mut R,
        writer: &mut W,
        prompt: &str,
        options: &[&str],
    ) -> io::Result<usize> {
        if options.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "select_menu requer ao menos uma opção",
            ));
        }

        writeln!(writer, "{}", prompt)?;
        for (idx, opt) in options.iter().enumerate() {
            writeln!(writer, "  {}. {}", idx + 1, opt)?;
        }

        loop {
            write!(writer, "Escolha (1-{}): ", options.len())?;
            writer.flush()?;
            let mut linha = String::new();
            let n = reader.read_line(&mut linha)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "EOF no menu de seleção",
                ));
            }
            if let Ok(num) = linha.trim().parse::<usize>() {
                if num >= 1 && num <= options.len() {
                    return Ok(num - 1);
                }
            }
            writeln!(writer, "Opção inválida. Digite um número entre 1 e {}.", options.len())?;
        }
    }

    /// Exibe menu numerado interativo em `stdin`/`stdout` retornando índice 0-based.
    #[inline]
    pub fn select_menu(prompt: &str, options: &[&str]) -> io::Result<usize> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut r = stdin.lock();
        let mut w = stdout.lock();
        select_menu_with(&mut r, &mut w, prompt, options)
    }

    /// Barra de progresso textual configurável para terminal (`stderr`) ou logs.
    #[derive(Debug, Clone)]
    pub struct ProgressBar {
        total: u64,
        current: u64,
        width: usize,
        label: String,
        mode: ColorMode,
    }

    impl ProgressBar {
        /// Cria uma barra de progresso com `total` unidades e largura `width` em caracteres.
        pub fn new(total: u64, width: usize) -> Self {
            ProgressBar {
                total,
                current: 0,
                width: width.max(1),
                label: String::new(),
                mode: ColorMode::Auto,
            }
        }

        /// Define o rótulo prefixado à barra.
        pub fn set_label(&mut self, label: &str) {
            self.label = label.to_string();
        }

        /// Define a política de cores ANSI da barra.
        pub fn set_color_mode(&mut self, mode: ColorMode) {
            self.mode = mode;
        }

        /// Atualiza a posição atual (limitada a `total`).
        pub fn update(&mut self, current: u64) {
            self.current = if self.total == 0 {
                0
            } else {
                current.min(self.total)
            };
        }

        /// Incrementa a posição atual em `delta`.
        pub fn inc(&mut self, delta: u64) {
            let novo = self.current.saturating_add(delta);
            self.update(novo);
        }

        /// Renderiza a barra como `String` determinística (sem `\r`).
        pub fn render(&self) -> String {
            let pct = if self.total == 0 {
                100.0
            } else {
                (self.current as f64 / self.total as f64) * 100.0
            };
            let preenchido = if self.total == 0 {
                self.width
            } else {
                (((self.current as u128) * (self.width as u128)) / (self.total as u128)) as usize
            };
            let vazio = self.width.saturating_sub(preenchido);
            let mut barra = String::with_capacity(self.width);
            for _ in 0..preenchido {
                barra.push('#');
            }
            for _ in 0..vazio {
                barra.push('-');
            }
            let barra_vis = style_text_stderr(&barra, Style::new().fg(Color::Green), self.mode);

            let mut out = String::new();
            if !self.label.is_empty() {
                let _ = write!(out, "{} ", self.label);
            }
            let _ = write!(
                out,
                "[{}] {:5.1}% ({}/{})",
                barra_vis, pct, self.current, self.total
            );
            out
        }

        /// Desenha a barra em `stderr`: usa `\r` in-place quando conectado a um TTY, ou
        /// imprime linha simples apenas quando concluído se redirecionado.
        pub fn draw(&self) -> io::Result<()> {
            let mut err = io::stderr().lock();
            if is_stderr_tty() {
                write!(err, "\r{}", self.render())?;
                err.flush()
            } else if self.current >= self.total {
                writeln!(err, "{}", self.render())?;
                err.flush()
            } else {
                Ok(())
            }
        }

        /// Marca a barra como 100% concluída e avança para a linha seguinte no TTY.
        pub fn finish(&mut self) -> io::Result<()> {
            self.current = self.total;
            let mut err = io::stderr().lock();
            if is_stderr_tty() {
                writeln!(err, "\r{}", self.render())?;
            } else {
                writeln!(err, "{}", self.render())?;
            }
            err.flush()
        }
    }

    /// Limpa a tela do terminal usando sequência ANSI somente se `stdout` for um TTY.
    pub fn clear_screen() -> io::Result<()> {
        if !is_stdout_tty() {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        out.write_all(b"\x1b[2J\x1b[H")?;
        out.flush()
    }
}

// ============================================================================
// 10F) concurrency_utils — Paralelismo com limite de threads, canais e pool
// ============================================================================
pub mod concurrency_utils {
    //! Primitivas ergonômicas de concorrência sobre `std::thread` e `std::sync`.
    //!
    //! **Regras de segurança operacional:**
    //! - Nunca cria um número ilimitado de threads (teto padrão [`MAX_DEFAULT_THREADS`] = 256).
    //! - Para cargas pequenas (`items.len() <= 1`) ou `max_threads <= 1`, executa diretamente de
    //!   forma sequencial sem custo de criação de threads.

    use std::fmt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    /// Teto máximo padrão para evitar criação excessiva de threads/descritores.
    pub const MAX_DEFAULT_THREADS: usize = 256;

    /// Erro em acesso a estado compartilhado ou envio em canal/pool encerrado.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ConcurrencyError {
        MutexPoisoned,
        ChannelClosed,
        InvalidConfig(String),
    }

    impl fmt::Display for ConcurrencyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ConcurrencyError::MutexPoisoned => {
                    write!(f, "ConcurrencyError: mutex envenenado após panic em thread")
                }
                ConcurrencyError::ChannelClosed => {
                    write!(f, "ConcurrencyError: canal ou pool de threads encerrado")
                }
                ConcurrencyError::InvalidConfig(msg) => {
                    write!(f, "ValueError: configuração inválida ({})", msg)
                }
            }
        }
    }

    impl std::error::Error for ConcurrencyError {}

    /// Mapeia `items` aplicando `f` em paralelo (até `max_threads` workers, limitado por
    /// [`MAX_DEFAULT_THREADS`]), preservando exatamente a ordem original dos elementos.
    /// Executa sequencialmente sem overhead de threads quando `items.len() <= 1` ou
    /// `max_threads <= 1`.
    pub fn parallel_map<T, U, F>(items: Vec<T>, max_threads: usize, f: F) -> Vec<U>
    where
        T: Send,
        U: Send,
        F: Fn(T) -> U + Sync,
    {
        let n = items.len();
        if n == 0 {
            return Vec::new();
        }
        if n == 1 || max_threads <= 1 {
            return items.into_iter().map(f).collect();
        }

        let num_workers = max_threads.clamp(1, MAX_DEFAULT_THREADS).min(n);
        let entradas: Vec<Mutex<Option<T>>> =
            items.into_iter().map(|x| Mutex::new(Some(x))).collect();
        let saidas: Vec<Mutex<Option<U>>> = (0..n).map(|_| Mutex::new(None)).collect();
        let proximo = AtomicUsize::new(0);
        let func = &f;

        std::thread::scope(|s| {
            for _ in 0..num_workers {
                s.spawn(|| loop {
                    let idx = proximo.fetch_add(1, Ordering::Relaxed);
                    if idx >= n {
                        break;
                    }
                    let val = entradas[idx]
                        .lock()
                        .expect("slot de entrada não envenenado")
                        .take()
                        .expect("índice único garantido por fetch_add");
                    let res = func(val);
                    *saidas[idx]
                        .lock()
                        .expect("slot de saída não envenenado") = Some(res);
                });
            }
        });

        saidas
            .into_iter()
            .map(|m| {
                m.into_inner()
                    .expect("mutex de saída")
                    .expect("todos os índices preenchidos")
            })
            .collect()
    }

    /// Executa `f(&item)` para cada elemento de `items` usando até `max_threads` workers.
    /// Executa sequencialmente quando `items.len() <= 1` ou `max_threads <= 1`.
    pub fn parallel_for_each<T, F>(items: &[T], max_threads: usize, f: F)
    where
        T: Sync,
        F: Fn(&T) + Sync,
    {
        let n = items.len();
        if n == 0 {
            return;
        }
        if n == 1 || max_threads <= 1 {
            for item in items {
                f(item);
            }
            return;
        }

        let num_workers = max_threads.clamp(1, MAX_DEFAULT_THREADS).min(n);
        let proximo = AtomicUsize::new(0);
        let func = &f;

        std::thread::scope(|s| {
            for _ in 0..num_workers {
                s.spawn(|| loop {
                    let idx = proximo.fetch_add(1, Ordering::Relaxed);
                    if idx >= n {
                        break;
                    }
                    func(&items[idx]);
                });
            }
        });
    }

    /// Wrapper ergonômico sobre `Arc<Mutex<T>>` que expõe falhas de envenenamento via `Result`.
    #[derive(Debug)]
    pub struct Shared<T> {
        inner: Arc<Mutex<T>>,
    }

    impl<T> Clone for Shared<T> {
        fn clone(&self) -> Self {
            Shared {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    impl<T> Shared<T> {
        /// Cria um novo valor compartilhado protegido por `Mutex`.
        #[inline]
        pub fn new(value: T) -> Self {
            Shared {
                inner: Arc::new(Mutex::new(value)),
            }
        }

        /// Clona o ponteiro compartilhado (`Arc::clone`).
        #[inline]
        pub fn clone_handle(&self) -> Self {
            self.clone()
        }

        /// Executa `f(&T)` sob o lock sem causar panic em mutex envenenado.
        pub fn try_with<R, F: FnOnce(&T) -> R>(&self, f: F) -> Result<R, ConcurrencyError> {
            let guard = self
                .inner
                .lock()
                .map_err(|_| ConcurrencyError::MutexPoisoned)?;
            Ok(f(&*guard))
        }

        /// Executa `f(&mut T)` sob o lock sem causar panic em mutex envenenado.
        pub fn try_with_mut<R, F: FnOnce(&mut T) -> R>(
            &self,
            f: F,
        ) -> Result<R, ConcurrencyError> {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| ConcurrencyError::MutexPoisoned)?;
            Ok(f(&mut *guard))
        }

        /// Obtém uma cópia do valor protegido.
        pub fn get_cloned(&self) -> Result<T, ConcurrencyError>
        where
            T: Clone,
        {
            self.try_with(Clone::clone)
        }

        /// Substitui o valor protegido.
        pub fn set(&self, val: T) -> Result<(), ConcurrencyError> {
            self.try_with_mut(|cur| *cur = val)
        }
    }

    /// Cria um canal síncrono com capacidade limitada (`mpsc::sync_channel`).
    #[inline]
    pub fn bounded_channel<T>(bound: usize) -> (mpsc::SyncSender<T>, mpsc::Receiver<T>) {
        mpsc::sync_channel(bound)
    }

    /// Cria um canal assíncrono ilimitado (`mpsc::channel`).
    #[inline]
    pub fn unbounded_channel<T>() -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
        mpsc::channel()
    }

    /// Limitador de taxa thread-safe (pacing por intervalo mínimo entre permissões).
    /// Libera o mutex interno antes de dormir, evitando bloquear reservas de slots futuros.
    #[derive(Debug)]
    pub struct RateLimiter {
        interval: Duration,
        next_slot: Mutex<Instant>,
    }

    impl RateLimiter {
        /// Cria um limitador que permite até `per_second` operações por segundo.
        pub fn new(per_second: f64) -> Result<Self, ConcurrencyError> {
            if !per_second.is_finite() || per_second <= 0.0 {
                return Err(ConcurrencyError::InvalidConfig(
                    "per_second deve ser finito e > 0".to_string(),
                ));
            }
            let interval = Duration::try_from_secs_f64(1.0 / per_second).map_err(|_| {
                ConcurrencyError::InvalidConfig("taxa por segundo fora do intervalo".to_string())
            })?;
            Ok(Self::from_interval(interval))
        }

        /// Cria um limitador com intervalo explícito entre permissões sucessivas.
        pub fn from_interval(interval: Duration) -> Self {
            RateLimiter {
                interval,
                next_slot: Mutex::new(Instant::now()),
            }
        }

        /// Bloqueia a thread atual se necessário até a próxima janela permitida.
        pub fn acquire(&self) {
            if self.interval.is_zero() {
                return;
            }
            let wait = {
                let mut next = match self.next_slot.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                let now = Instant::now();
                if *next <= now {
                    *next = now + self.interval;
                    Duration::ZERO
                } else {
                    let diff = *next - now;
                    *next += self.interval;
                    diff
                }
            };
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
        }
    }

    type Job = Box<dyn FnOnce() + Send + 'static>;

    /// Pool fixo de worker threads com encerramento gracioso no `Drop`.
    pub struct ThreadPool {
        workers: Vec<JoinHandle<()>>,
        sender: Option<mpsc::Sender<Job>>,
    }

    impl ThreadPool {
        /// Cria um pool com `threads` workers (limitado ao intervalo `1..=MAX_DEFAULT_THREADS`).
        pub fn new(threads: usize) -> Self {
            let n = threads.clamp(1, MAX_DEFAULT_THREADS);
            let (tx, rx) = mpsc::channel::<Job>();
            let rx = Arc::new(Mutex::new(rx));
            let mut workers = Vec::with_capacity(n);

            for _ in 0..n {
                let rx_clone = Arc::clone(&rx);
                let handle = std::thread::spawn(move || loop {
                    let job = {
                        let guard = match rx_clone.lock() {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        guard.recv()
                    };
                    match job {
                        Ok(task) => task(),
                        Err(_) => break,
                    }
                });
                workers.push(handle);
            }

            ThreadPool {
                workers,
                sender: Some(tx),
            }
        }

        /// Quantidade de worker threads no pool.
        #[inline]
        pub fn worker_count(&self) -> usize {
            self.workers.len()
        }

        /// Submete uma tarefa *fire-and-forget* ao pool.
        pub fn execute<F>(&self, f: F) -> Result<(), ConcurrencyError>
        where
            F: FnOnce() + Send + 'static,
        {
            match &self.sender {
                Some(tx) => tx
                    .send(Box::new(f))
                    .map_err(|_| ConcurrencyError::ChannelClosed),
                None => Err(ConcurrencyError::ChannelClosed),
            }
        }

        /// Submete uma tarefa que produz valor `R`, devolvendo um `Receiver<R>` para aguardar o resultado.
        pub fn submit<R, F>(&self, f: F) -> Result<mpsc::Receiver<R>, ConcurrencyError>
        where
            R: Send + 'static,
            F: FnOnce() -> R + Send + 'static,
        {
            let (res_tx, res_rx) = mpsc::sync_channel(1);
            self.execute(move || {
                let val = f();
                let _ = res_tx.send(val);
            })?;
            Ok(res_rx)
        }
    }

    impl Drop for ThreadPool {
        fn drop(&mut self) {
            drop(self.sender.take());
            for handle in self.workers.drain(..) {
                let _ = handle.join();
            }
        }
    }
}

// ============================================================================
// 9) Rede & CyberSec Networking — TCP/UDP, CIDR IPv4/IPv6, Scanner, Packet, PCAP
// ============================================================================
pub mod net_utils {
    //! Wrappers ergonômicos sobre `std::net` (`TcpClient`, `TcpServer`, `UdpSock`),
    //! primitivas estilo `pwntools`, cálculo CIDR IPv4 e IPv6 (`ipaddress`), *port scanner*
    //! concorrente limitado com rate limiting, construção e dissecação defensiva de pacotes
    //! IPv4/TCP e IPv4/UDP com checksum RFC 1071, geração/leitura em streaming `.pcap` e
    //! inspeção passiva de `/proc/net/*` no Linux.
    //!
    //! **Limitações técnicas da `std`:**
    //! - `PcapWriter` / `StreamingPcapWriter` gravam pacotes sintéticos ou capturados em camada de
    //!   aplicação no formato Libpcap v2.4; a `std` pura **não** realiza captura ao vivo de
    //!   interfaces de rede nem envia raw sockets sem FFI/libpcap.

    use super::concurrency_utils::RateLimiter;
    use std::convert::TryFrom;
    use std::fmt;
    use std::fs::{self, File};
    use std::io::{self, BufReader, BufWriter, Read, Write};
    use std::net::{
        self, IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, ToSocketAddrs,
    };
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    /// Teto máximo de threads criadas simultaneamente em [`tcp_scan`] / [`tcp_scan_with_config`].
    pub const MAX_SCAN_THREADS: usize = 256;

    fn para_duracao(secs: Option<f64>) -> io::Result<Option<Duration>> {
        match secs {
            None => Ok(None),
            Some(s) => Duration::try_from_secs_f64(s).map(Some).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "timeout inválido (negativo/NaN/infinito)",
                )
            }),
        }
    }

    /// Cliente TCP ergonômico sobre `std::net::TcpStream` com primitivas estilo `pwntools`.
    #[derive(Debug)]
    pub struct TcpClient {
        stream: net::TcpStream,
    }

    impl TcpClient {
        pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            Ok(TcpClient {
                stream: net::TcpStream::connect(addr)?,
            })
        }

        pub fn connect_timeout<A: ToSocketAddrs>(addr: A, secs: f64) -> io::Result<Self> {
            let dur = para_duracao(Some(secs))?.unwrap_or_default();
            let mut ultimo = io::Error::new(io::ErrorKind::NotFound, "nenhum endereço resolvido");
            for a in addr.to_socket_addrs()? {
                match net::TcpStream::connect_timeout(&a, dur) {
                    Ok(stream) => return Ok(TcpClient { stream }),
                    Err(e) => ultimo = e,
                }
            }
            Err(ultimo)
        }

        /// Tenta conectar repetidamente até `retries` vezes adicionais com espera `backoff_secs`
        /// entre falhas.
        pub fn connect_with_retry<A: ToSocketAddrs>(
            addr: A,
            timeout_secs: f64,
            retries: usize,
            backoff_secs: f64,
        ) -> io::Result<Self> {
            let backoff = para_duracao(Some(backoff_secs))?.unwrap_or_default();
            let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();
            if addrs.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "nenhum endereço resolvido",
                ));
            }
            let dur = para_duracao(Some(timeout_secs))?.unwrap_or_default();
            let mut ultimo = io::Error::new(io::ErrorKind::Other, "falha de conexão");

            for tentativa in 0..=retries {
                for &a in &addrs {
                    match net::TcpStream::connect_timeout(&a, dur) {
                        Ok(stream) => return Ok(TcpClient { stream }),
                        Err(e) => ultimo = e,
                    }
                }
                if tentativa < retries && !backoff.is_zero() {
                    std::thread::sleep(backoff);
                }
            }
            Err(ultimo)
        }

        pub fn send<D: AsRef<[u8]>>(&mut self, dados: D) -> io::Result<()> {
            self.stream.write_all(dados.as_ref())
        }

        /// Força a descarga de eventuais buffers de escrita.
        #[inline]
        pub fn flush(&mut self) -> io::Result<()> {
            self.stream.flush()
        }

        /// Recebe até `max` bytes (trata `EINTR`; limita alocação inicial por chamada a 1 MiB —
        /// use [`recv_capped`](Self::recv_capped) para teto explícito maior ou menor).
        pub fn recv(&mut self, max: usize) -> io::Result<Vec<u8>> {
            if max == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; max.min(1_048_576)];
            loop {
                match self.stream.read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        return Ok(buf);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        /// Recebe até `max` bytes validando contra um teto máximo `cap` antes de alocar.
        pub fn recv_capped(&mut self, max: usize, cap: usize) -> io::Result<Vec<u8>> {
            if max > cap {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("tamanho solicitado ({}) excede teto ({})", max, cap),
                ));
            }
            if max == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; max];
            loop {
                match self.stream.read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        return Ok(buf);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        /// Lê para o slice fornecido pelo chamador (zero alocação), tratando `EINTR`.
        pub fn recv_into(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            loop {
                match self.stream.read(buf) {
                    Ok(n) => return Ok(n),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn recv_exact(&mut self, n: usize) -> io::Result<Vec<u8>> {
            let mut buf = vec![0u8; n];
            self.stream.read_exact(&mut buf)?;
            Ok(buf)
        }

        /// Lê exatamente `n` bytes garantindo que `n <= cap` antes de alocar.
        pub fn recv_exact_capped(&mut self, n: usize, cap: usize) -> io::Result<Vec<u8>> {
            if n > cap {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("recv_exact_capped: {} excede teto {}", n, cap),
                ));
            }
            self.recv_exact(n)
        }

        pub fn recv_string(&mut self, max: usize) -> io::Result<String> {
            Ok(String::from_utf8_lossy(&self.recv(max)?).into_owned())
        }

        /// Espia até `max` bytes sem removê-los do buffer do kernel (`MSG_PEEK`).
        pub fn peek(&self, max: usize) -> io::Result<Vec<u8>> {
            if max == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; max.min(1_048_576)];
            let n = self.peek_into(&mut buf)?;
            buf.truncate(n);
            Ok(buf)
        }

        /// Espia bytes diretamente para `buf` sem consumi-los do socket (`MSG_PEEK`).
        pub fn peek_into(&self, buf: &mut [u8]) -> io::Result<usize> {
            loop {
                match self.stream.peek(buf) {
                    Ok(n) => return Ok(n),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        /// Encerra uma ou ambas as metades do fluxo TCP (`Shutdown::Read`, `Write`, `Both`).
        #[inline]
        pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
            self.stream.shutdown(how)
        }

        pub fn set_timeout(&mut self, secs: Option<f64>) -> io::Result<()> {
            let d = para_duracao(secs)?;
            self.stream.set_read_timeout(d)?;
            self.stream.set_write_timeout(d)
        }

        pub fn set_read_timeout(&self, secs: Option<f64>) -> io::Result<()> {
            let d = para_duracao(secs)?;
            self.stream.set_read_timeout(d)
        }

        pub fn set_write_timeout(&self, secs: Option<f64>) -> io::Result<()> {
            let d = para_duracao(secs)?;
            self.stream.set_write_timeout(d)
        }

        #[inline]
        pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
            self.stream.read_timeout()
        }

        #[inline]
        pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
            self.stream.write_timeout()
        }

        #[inline]
        pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
            self.stream.set_nonblocking(nonblocking)
        }

        pub fn set_nodelay(&mut self, ligado: bool) -> io::Result<()> {
            self.stream.set_nodelay(ligado)
        }

        #[inline]
        pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
            self.stream.set_ttl(ttl)
        }

        #[inline]
        pub fn ttl(&self) -> io::Result<u32> {
            self.stream.ttl()
        }

        pub fn peer_addr(&self) -> io::Result<SocketAddr> {
            self.stream.peer_addr()
        }

        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.stream.local_addr()
        }

        #[inline]
        pub fn stream(&self) -> &net::TcpStream {
            &self.stream
        }

        #[inline]
        pub fn stream_mut(&mut self) -> &mut net::TcpStream {
            &mut self.stream
        }

        pub fn close(self) -> io::Result<()> {
            self.stream.shutdown(Shutdown::Both)
        }

        // ---------------- Primitivas estilo pwntools ----------------

        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(TcpClient {
                stream: self.stream.try_clone()?,
            })
        }

        pub fn sendline<D: AsRef<[u8]>>(&mut self, dados: D) -> io::Result<()> {
            self.stream.write_all(dados.as_ref())?;
            self.stream.write_all(b"\n")?;
            self.stream.flush()
        }

        /// Lê do socket byte a byte (zero *over-read*) até encontrar `delim`, atingir
        /// `max_bytes` ou encontrar EOF (devolve o que acumulou em EOF ou `max_bytes`).
        /// Para exigir que `delim` tenha sido encontrado, use [`recvuntil_strict`](Self::recvuntil_strict).
        pub fn recvuntil<D: AsRef<[u8]>>(
            &mut self,
            delim: D,
            max_bytes: usize,
        ) -> io::Result<Vec<u8>> {
            let padrao = delim.as_ref();
            let mut acumulado = Vec::new();
            if padrao.is_empty() || max_bytes == 0 {
                return Ok(acumulado);
            }
            let mut byte = [0u8; 1];
            while acumulado.len() < max_bytes {
                match self.stream.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => {
                        acumulado.push(byte[0]);
                        if acumulado.len() >= padrao.len()
                            && &acumulado[acumulado.len() - padrao.len()..] == padrao
                        {
                            break;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
            Ok(acumulado)
        }

        /// Variante estrita de [`recvuntil`](Self::recvuntil): retorna erro `UnexpectedEof` se o
        /// peer fechar antes de `delim` ou `InvalidData` se atingir `max_bytes` sem encontrá-lo.
        pub fn recvuntil_strict<D: AsRef<[u8]>>(
            &mut self,
            delim: D,
            max_bytes: usize,
        ) -> io::Result<Vec<u8>> {
            let padrao = delim.as_ref();
            if padrao.is_empty() {
                return Ok(Vec::new());
            }
            if max_bytes == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "max_bytes deve ser > 0 em recvuntil_strict",
                ));
            }
            let mut acumulado = Vec::new();
            let mut byte = [0u8; 1];
            while acumulado.len() < max_bytes {
                match self.stream.read(&mut byte) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "EOF antes de encontrar delimitador em recvuntil_strict",
                        ));
                    }
                    Ok(_) => {
                        acumulado.push(byte[0]);
                        if acumulado.len() >= padrao.len()
                            && &acumulado[acumulado.len() - padrao.len()..] == padrao
                        {
                            return Ok(acumulado);
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "limite max_bytes atingido sem encontrar delimitador",
            ))
        }

        #[inline]
        pub fn recvline(&mut self, max_bytes: usize) -> io::Result<Vec<u8>> {
            self.recvuntil(b"\n", max_bytes)
        }

        pub fn recvline_string(&mut self, max_bytes: usize) -> io::Result<String> {
            let bytes = self.recvline(max_bytes)?;
            let mut s = String::from_utf8_lossy(&bytes).into_owned();
            if s.ends_with('\n') {
                s.pop();
                if s.ends_with('\r') {
                    s.pop();
                }
            }
            Ok(s)
        }

        pub fn sendafter<D: AsRef<[u8]>, P: AsRef<[u8]>>(
            &mut self,
            delim: D,
            payload: P,
            max_bytes: usize,
        ) -> io::Result<Vec<u8>> {
            let recebido = self.recvuntil(delim, max_bytes)?;
            self.send(payload)?;
            Ok(recebido)
        }

        pub fn sendlineafter<D: AsRef<[u8]>, P: AsRef<[u8]>>(
            &mut self,
            delim: D,
            payload: P,
            max_bytes: usize,
        ) -> io::Result<Vec<u8>> {
            let recebido = self.recvuntil(delim, max_bytes)?;
            self.sendline(payload)?;
            Ok(recebido)
        }

        pub fn interactive(&mut self) -> io::Result<()> {
            let mut leitor = self.stream.try_clone()?;
            let mut escritor = self.stream.try_clone()?;

            std::thread::scope(|s| {
                let t_out = s.spawn(move || -> io::Result<()> {
                    let mut buf = [0u8; 1024];
                    let mut stdout = io::stdout();
                    loop {
                        match leitor.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                stdout.write_all(&buf[..n])?;
                                stdout.flush()?;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                            Err(_) => break,
                        }
                    }
                    Ok(())
                });

                let mut stdin = io::stdin();
                let mut buf_in = [0u8; 1024];
                loop {
                    match stdin.read(&mut buf_in) {
                        Ok(0) => {
                            let _ = escritor.shutdown(Shutdown::Write);
                            break;
                        }
                        Ok(n) => {
                            if escritor.write_all(&buf_in[..n]).is_err() {
                                break;
                            }
                            let _ = escritor.flush();
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                let _ = t_out.join();
            });
            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct TcpServer {
        listener: net::TcpListener,
    }

    impl TcpServer {
        pub fn listen<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            Ok(TcpServer {
                listener: net::TcpListener::bind(addr)?,
            })
        }

        pub fn accept(&self) -> io::Result<(TcpClient, SocketAddr)> {
            let (stream, addr) = self.listener.accept()?;
            Ok((TcpClient { stream }, addr))
        }

        pub fn incoming(&self) -> impl Iterator<Item = io::Result<TcpClient>> + '_ {
            self.listener
                .incoming()
                .map(|r| r.map(|stream| TcpClient { stream }))
        }

        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.listener.local_addr()
        }

        #[inline]
        pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
            self.listener.set_nonblocking(nonblocking)
        }

        #[inline]
        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(TcpServer {
                listener: self.listener.try_clone()?,
            })
        }

        #[inline]
        pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
            self.listener.set_ttl(ttl)
        }

        #[inline]
        pub fn ttl(&self) -> io::Result<u32> {
            self.listener.ttl()
        }

        #[inline]
        pub fn listener(&self) -> &net::TcpListener {
            &self.listener
        }
    }

    #[derive(Debug)]
    pub struct UdpSock {
        socket: net::UdpSocket,
    }

    impl UdpSock {
        pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            Ok(UdpSock {
                socket: net::UdpSocket::bind(addr)?,
            })
        }

        /// Conecta o socket UDP a um peer remoto padrão para usar [`send`](Self::send) e
        /// [`recv`](Self::recv).
        #[inline]
        pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
            self.socket.connect(addr)
        }

        #[inline]
        pub fn send<D: AsRef<[u8]>>(&self, dados: D) -> io::Result<usize> {
            self.socket.send(dados.as_ref())
        }

        /// Recebe datagrama de socket UDP conectado (limita buffer a 65536 bytes).
        pub fn recv(&self, max: usize) -> io::Result<Vec<u8>> {
            if max == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; max.min(65_536)];
            loop {
                match self.socket.recv(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        return Ok(buf);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn recv_into(&self, buf: &mut [u8]) -> io::Result<usize> {
            loop {
                match self.socket.recv(buf) {
                    Ok(n) => return Ok(n),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn send_to<D: AsRef<[u8]>, A: ToSocketAddrs>(
            &self,
            dados: D,
            addr: A,
        ) -> io::Result<usize> {
            self.socket.send_to(dados.as_ref(), addr)
        }

        pub fn recv_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
            if max == 0 {
                let mut dummy = [0u8; 0];
                let (_, origem) = self.socket.recv_from(&mut dummy)?;
                return Ok((Vec::new(), origem));
            }
            let mut buf = vec![0u8; max.min(65_536)];
            loop {
                match self.socket.recv_from(&mut buf) {
                    Ok((n, origem)) => {
                        buf.truncate(n);
                        return Ok((buf, origem));
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn recv_from_into(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            loop {
                match self.socket.recv_from(buf) {
                    Ok(res) => return Ok(res),
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn peek(&self, max: usize) -> io::Result<Vec<u8>> {
            if max == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; max.min(65_536)];
            loop {
                match self.socket.peek(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        return Ok(buf);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn peek_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
            if max == 0 {
                let mut dummy = [0u8; 0];
                let (_, origem) = self.socket.peek_from(&mut dummy)?;
                return Ok((Vec::new(), origem));
            }
            let mut buf = vec![0u8; max.min(65_536)];
            loop {
                match self.socket.peek_from(&mut buf) {
                    Ok((n, origem)) => {
                        buf.truncate(n);
                        return Ok((buf, origem));
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn set_timeout(&self, secs: Option<f64>) -> io::Result<()> {
            let d = para_duracao(secs)?;
            self.socket.set_read_timeout(d)?;
            self.socket.set_write_timeout(d)
        }

        #[inline]
        pub fn set_read_timeout(&self, secs: Option<f64>) -> io::Result<()> {
            self.socket.set_read_timeout(para_duracao(secs)?)
        }

        #[inline]
        pub fn set_write_timeout(&self, secs: Option<f64>) -> io::Result<()> {
            self.socket.set_write_timeout(para_duracao(secs)?)
        }

        #[inline]
        pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
            self.socket.read_timeout()
        }

        #[inline]
        pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
            self.socket.write_timeout()
        }

        #[inline]
        pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
            self.socket.set_nonblocking(nonblocking)
        }

        #[inline]
        pub fn set_broadcast(&self, ligado: bool) -> io::Result<()> {
            self.socket.set_broadcast(ligado)
        }

        #[inline]
        pub fn broadcast(&self) -> io::Result<bool> {
            self.socket.broadcast()
        }

        #[inline]
        pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
            self.socket.set_ttl(ttl)
        }

        #[inline]
        pub fn ttl(&self) -> io::Result<u32> {
            self.socket.ttl()
        }

        #[inline]
        pub fn join_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> io::Result<()> {
            self.socket.join_multicast_v4(multiaddr, interface)
        }

        #[inline]
        pub fn leave_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> io::Result<()> {
            self.socket.leave_multicast_v4(multiaddr, interface)
        }

        #[inline]
        pub fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
            self.socket.join_multicast_v6(multiaddr, interface)
        }

        #[inline]
        pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
            self.socket.leave_multicast_v6(multiaddr, interface)
        }

        #[inline]
        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(UdpSock {
                socket: self.socket.try_clone()?,
            })
        }

        #[inline]
        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.socket.local_addr()
        }

        #[inline]
        pub fn peer_addr(&self) -> io::Result<SocketAddr> {
            self.socket.peer_addr()
        }

        #[inline]
        pub fn socket(&self) -> &net::UdpSocket {
            &self.socket
        }
    }

    pub fn getaddrinfo(host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
        Ok((host, port).to_socket_addrs()?.collect())
    }

    pub fn gethostbyname(host: &str) -> io::Result<Ipv4Addr> {
        (host, 0)
            .to_socket_addrs()?
            .find_map(|a| match a {
                SocketAddr::V4(v4) => Some(*v4.ip()),
                SocketAddr::V6(_) => None,
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("nenhum IPv4 para '{}'", host),
                )
            })
    }

    // ========================================================================
    // CIDR IPv4, CIDR IPv6, Intervalos IP e Port Ranges
    // ========================================================================

    /// Bloco CIDR IPv4 com iteração lazy ($O(1)$ em memória) e verificação de sub-redes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Ipv4Cidr {
        base_ip: u32,
        prefix: u8,
        atual: u64,
        fim_exclusivo: u64,
    }

    impl Ipv4Cidr {
        pub fn parse(cidr: &str) -> Result<Self, String> {
            let s = cidr.trim();
            let (ip_str, pref_str) = match s.split_once('/') {
                Some((i, p)) => (i.trim(), p.trim()),
                None => (s, "32"),
            };
            let ip: Ipv4Addr = ip_str
                .parse()
                .map_err(|_| format!("ValueError: IPv4 inválido em CIDR '{}'", cidr))?;
            let prefix: u8 = pref_str
                .parse()
                .map_err(|_| format!("ValueError: prefixo CIDR inválido em '{}'", cidr))?;
            if prefix > 32 {
                return Err(format!(
                    "ValueError: prefixo CIDR fora do intervalo 0..=32 ({})",
                    prefix
                ));
            }
            let mask = if prefix == 0 {
                0u32
            } else {
                !0u32 << (32 - prefix)
            };
            let base_ip = u32::from(ip) & mask;
            let qtd = 1u64 << (32 - prefix as u64);
            Ok(Ipv4Cidr {
                base_ip,
                prefix,
                atual: base_ip as u64,
                fim_exclusivo: (base_ip as u64) + qtd,
            })
        }

        #[inline]
        pub fn prefix_len(&self) -> u8 {
            self.prefix
        }

        #[inline]
        pub fn netmask(&self) -> Ipv4Addr {
            let mask = if self.prefix == 0 {
                0u32
            } else {
                !0u32 << (32 - self.prefix)
            };
            Ipv4Addr::from(mask)
        }

        #[inline]
        pub fn network(&self) -> Ipv4Addr {
            Ipv4Addr::from(self.base_ip)
        }

        #[inline]
        pub fn broadcast(&self) -> Ipv4Addr {
            let host_mask = if self.prefix == 32 {
                0u32
            } else if self.prefix == 0 {
                !0u32
            } else {
                (1u32 << (32 - self.prefix)) - 1
            };
            Ipv4Addr::from(self.base_ip | host_mask)
        }

        #[inline]
        pub fn contains(&self, ip: Ipv4Addr) -> bool {
            let mask = u32::from(self.netmask());
            (u32::from(ip) & mask) == self.base_ip
        }

        #[inline]
        pub fn total_addresses(&self) -> u64 {
            1u64 << (32 - self.prefix as u64)
        }

        /// Primeiro endereço de host utilizável (exclui rede quando `prefix <= 30`).
        #[inline]
        pub fn first_host(&self) -> Option<Ipv4Addr> {
            if self.prefix <= 30 {
                Some(Ipv4Addr::from(self.base_ip + 1))
            } else {
                Some(self.network())
            }
        }

        /// Último endereço de host utilizável (exclui broadcast quando `prefix <= 30`).
        #[inline]
        pub fn last_host(&self) -> Option<Ipv4Addr> {
            if self.prefix <= 30 {
                Some(Ipv4Addr::from(u32::from(self.broadcast()) - 1))
            } else {
                Some(self.broadcast())
            }
        }

        /// Retorna o `index`-ésimo IPv4 dentro do bloco em $O(1)$.
        #[inline]
        pub fn nth_ip(&self, index: u64) -> Option<Ipv4Addr> {
            if index >= self.total_addresses() {
                None
            } else {
                Some(Ipv4Addr::from(((self.base_ip as u64) + index) as u32))
            }
        }

        /// Verifica se há sobreposição entre `self` e `other`.
        #[inline]
        pub fn overlaps(&self, other: &Self) -> bool {
            self.contains(other.network()) || other.contains(self.network())
        }

        /// Retorna `true` se `self` está inteiramente contido dentro de `other`.
        #[inline]
        pub fn is_subnet_of(&self, other: &Self) -> bool {
            self.prefix >= other.prefix && other.contains(self.network())
        }

        /// Retorna `true` se `self` contém inteiramente `other`.
        #[inline]
        pub fn is_supernet_of(&self, other: &Self) -> bool {
            other.is_subnet_of(self)
        }

        #[inline]
        pub fn hosts(&self) -> Self {
            if self.prefix <= 30 {
                Ipv4Cidr {
                    base_ip: self.base_ip,
                    prefix: self.prefix,
                    atual: (self.base_ip as u64) + 1,
                    fim_exclusivo: self.fim_exclusivo.saturating_sub(1),
                }
            } else {
                Ipv4Cidr {
                    base_ip: self.base_ip,
                    prefix: self.prefix,
                    atual: self.base_ip as u64,
                    fim_exclusivo: self.fim_exclusivo,
                }
            }
        }
    }

    impl Iterator for Ipv4Cidr {
        type Item = Ipv4Addr;

        #[inline]
        fn next(&mut self) -> Option<Ipv4Addr> {
            if self.atual >= self.fim_exclusivo {
                return None;
            }
            let ip = Ipv4Addr::from(self.atual as u32);
            self.atual += 1;
            Some(ip)
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let restante = self.fim_exclusivo.saturating_sub(self.atual);
            match usize::try_from(restante) {
                Ok(v) => (v, Some(v)),
                Err(_) => (usize::MAX, None),
            }
        }
    }

    /// Iterador lazy sobre um intervalo arbitrário fechado de endereços IPv4 `[start..=end]`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Ipv4Range {
        atual: u64,
        fim_inclusivo: u64,
    }

    impl Ipv4Range {
        /// Cria um intervalo inclusivo `start..=end` (erro se `start > end`).
        pub fn new(start: Ipv4Addr, end: Ipv4Addr) -> Result<Self, String> {
            let s = u32::from(start) as u64;
            let e = u32::from(end) as u64;
            if s > e {
                return Err(format!("ValueError: intervalo IPv4 invertido {}..={}", start, end));
            }
            Ok(Ipv4Range {
                atual: s,
                fim_inclusivo: e,
            })
        }

        /// Verifica em $O(1)$ se `ip` pertence ao intervalo.
        #[inline]
        pub fn contains(&self, ip: Ipv4Addr) -> bool {
            let x = u32::from(ip) as u64;
            x >= self.atual && x <= self.fim_inclusivo
        }

        #[inline]
        pub fn len_u64(&self) -> u64 {
            if self.atual > self.fim_inclusivo {
                0
            } else {
                self.fim_inclusivo - self.atual + 1
            }
        }
    }

    impl Iterator for Ipv4Range {
        type Item = Ipv4Addr;

        #[inline]
        fn next(&mut self) -> Option<Ipv4Addr> {
            if self.atual > self.fim_inclusivo {
                return None;
            }
            let ip = Ipv4Addr::from(self.atual as u32);
            self.atual += 1;
            Some(ip)
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let rem = self.len_u64();
            match usize::try_from(rem) {
                Ok(v) => (v, Some(v)),
                Err(_) => (usize::MAX, None),
            }
        }
    }

    /// Bloco CIDR IPv6 (ex.: `"2001:db8::/32"`) com suporte completo de `/0` até `/128` sem
    /// overflow e iteração lazy $O(1)$ em memória.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Ipv6Cidr {
        base_ip: u128,
        prefix: u8,
        offset: u128,
        done: bool,
    }

    impl Ipv6Cidr {
        /// Interpreta notação `"ADDR/PREFIX"` (ou `"ADDR"` assumindo `/128`).
        pub fn parse(cidr: &str) -> Result<Self, String> {
            let s = cidr.trim();
            let (ip_str, pref_str) = match s.split_once('/') {
                Some((i, p)) => (i.trim(), p.trim()),
                None => (s, "128"),
            };
            let ip: Ipv6Addr = ip_str
                .parse()
                .map_err(|_| format!("ValueError: IPv6 inválido em CIDR '{}'", cidr))?;
            let prefix: u8 = pref_str
                .parse()
                .map_err(|_| format!("ValueError: prefixo IPv6 inválido em '{}'", cidr))?;
            if prefix > 128 {
                return Err(format!(
                    "ValueError: prefixo IPv6 fora do intervalo 0..=128 ({})",
                    prefix
                ));
            }
            let mask = if prefix == 0 {
                0u128
            } else {
                !0u128 << (128 - prefix)
            };
            let base_ip = u128::from(ip) & mask;
            Ok(Ipv6Cidr {
                base_ip,
                prefix,
                offset: 0,
                done: false,
            })
        }

        #[inline]
        pub const fn prefix_len(&self) -> u8 {
            self.prefix
        }

        #[inline]
        pub fn netmask(&self) -> Ipv6Addr {
            let mask = if self.prefix == 0 {
                0u128
            } else {
                !0u128 << (128 - self.prefix)
            };
            Ipv6Addr::from(mask)
        }

        #[inline]
        pub fn network(&self) -> Ipv6Addr {
            Ipv6Addr::from(self.base_ip)
        }

        #[inline]
        pub fn last_address(&self) -> Ipv6Addr {
            let host_mask = if self.prefix == 128 {
                0u128
            } else if self.prefix == 0 {
                !0u128
            } else {
                (1u128 << (128 - self.prefix)) - 1
            };
            Ipv6Addr::from(self.base_ip | host_mask)
        }

        #[inline]
        pub fn contains(&self, ip: Ipv6Addr) -> bool {
            let mask = if self.prefix == 0 {
                0u128
            } else {
                !0u128 << (128 - self.prefix)
            };
            (u128::from(ip) & mask) == self.base_ip
        }

        /// Total de endereços (`None` para `/0`, pois $2^{128}$ excede `u128::MAX`).
        #[inline]
        pub fn total_addresses(&self) -> Option<u128> {
            if self.prefix == 0 {
                None
            } else {
                Some(1u128 << (128 - self.prefix))
            }
        }
    }

    impl Iterator for Ipv6Cidr {
        type Item = Ipv6Addr;

        #[inline]
        fn next(&mut self) -> Option<Ipv6Addr> {
            if self.done {
                return None;
            }
            let ip = Ipv6Addr::from(self.base_ip + self.offset);
            let max_offset = if self.prefix == 0 {
                u128::MAX
            } else {
                (1u128 << (128 - self.prefix)) - 1
            };
            if self.offset >= max_offset {
                self.done = true;
            } else {
                self.offset += 1;
            }
            Some(ip)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            if self.done {
                return (0, Some(0));
            }
            if self.prefix == 0 {
                return (usize::MAX, None);
            }
            let total = 1u128 << (128 - self.prefix);
            let rem = total.saturating_sub(self.offset);
            match usize::try_from(rem) {
                Ok(v) => (v, Some(v)),
                Err(_) => (usize::MAX, None),
            }
        }
    }

    /// Bloco CIDR unificado (IPv4 ou IPv6).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum IpCidr {
        V4(Ipv4Cidr),
        V6(Ipv6Cidr),
    }

    impl IpCidr {
        pub fn parse(cidr: &str) -> Result<Self, String> {
            let s = cidr.trim();
            let ip_part = s.split_once('/').map_or(s, |(i, _)| i.trim());
            match ip_part.parse::<IpAddr>() {
                Ok(IpAddr::V4(_)) => Ok(IpCidr::V4(Ipv4Cidr::parse(s)?)),
                Ok(IpAddr::V6(_)) => Ok(IpCidr::V6(Ipv6Cidr::parse(s)?)),
                Err(_) => Err(format!("ValueError: endereço CIDR inválido '{}'", cidr)),
            }
        }

        pub fn contains(&self, ip: IpAddr) -> bool {
            match (self, ip) {
                (IpCidr::V4(net), IpAddr::V4(v4)) => net.contains(v4),
                (IpCidr::V6(net), IpAddr::V6(v6)) => net.contains(v6),
                _ => false,
            }
        }

        #[inline]
        pub fn prefix_len(&self) -> u8 {
            match self {
                IpCidr::V4(n) => n.prefix_len(),
                IpCidr::V6(n) => n.prefix_len(),
            }
        }
    }

    #[inline]
    pub fn parse_cidr(cidr: &str) -> Result<Ipv4Cidr, String> {
        Ipv4Cidr::parse(cidr)
    }

    #[inline]
    pub fn parse_cidr_v6(cidr: &str) -> Result<Ipv6Cidr, String> {
        Ipv6Cidr::parse(cidr)
    }

    pub fn parse_ports(spec: &str) -> Result<Vec<u16>, String> {
        let mut portas = Vec::new();
        for parte in spec.split(',') {
            let item = parte.trim();
            if item.is_empty() {
                continue;
            }
            if let Some((ini_s, fim_s)) = item.split_once('-') {
                let ini: u16 = ini_s
                    .trim()
                    .parse()
                    .map_err(|_| format!("ValueError: porta inicial inválida em '{}'", item))?;
                let fim: u16 = fim_s
                    .trim()
                    .parse()
                    .map_err(|_| format!("ValueError: porta final inválida em '{}'", item))?;
                if ini == 0 || fim == 0 || ini > fim {
                    return Err(format!(
                        "ValueError: intervalo de portas inválido '{}' (use 1..=65535 com ini <= fim)",
                        item
                    ));
                }
                for p in ini..=fim {
                    portas.push(p);
                }
            } else {
                let p: u16 = item
                    .parse()
                    .map_err(|_| format!("ValueError: porta inválida '{}'", item))?;
                if p == 0 {
                    return Err("ValueError: porta 0 não é válida em parse_ports".to_string());
                }
                portas.push(p);
            }
        }
        portas.sort_unstable();
        portas.dedup();
        Ok(portas)
    }

    // ========================================================================
    // Port Scanner & Banner Grabber Concorrente Limitado
    // ========================================================================

    #[derive(Debug, Clone, PartialEq)]
    pub struct PortScanResult {
        pub host: String,
        pub port: u16,
        pub open: bool,
        pub banner: Option<String>,
        pub latency_ms: f64,
    }

    /// Estado detalhado de uma porta sondada.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PortState {
        Open,
        Closed,
        TimedOut,
        Unreachable,
        Error(String),
    }

    /// Resultado estruturado do scanner v2.0 com distinção de erro/timeout.
    #[derive(Debug, Clone, PartialEq)]
    pub struct DetailedPortScanResult {
        pub host: String,
        pub port: u16,
        pub state: PortState,
        pub banner: Option<String>,
        pub latency_ms: f64,
        pub error_kind: Option<io::ErrorKind>,
    }

    /// Configuração de scanner TCP concorrente com teto de threads e rate limiting opcional.
    #[derive(Debug, Clone)]
    pub struct ScanConfig {
        pub timeout_secs: f64,
        pub threads: usize,
        pub grab_banner: bool,
        pub banner_max_bytes: usize,
        pub rate_limit_per_sec: Option<f64>,
    }

    impl Default for ScanConfig {
        fn default() -> Self {
            ScanConfig {
                timeout_secs: 1.0,
                threads: 32,
                grab_banner: false,
                banner_max_bytes: 256,
                rate_limit_per_sec: None,
            }
        }
    }

    pub fn tcp_probe(
        host: &str,
        port: u16,
        timeout_secs: f64,
        grab_banner: bool,
    ) -> PortScanResult {
        let det = tcp_probe_detailed(host, port, timeout_secs, grab_banner, 256);
        PortScanResult {
            host: det.host,
            port: det.port,
            open: det.state == PortState::Open,
            banner: det.banner,
            latency_ms: det.latency_ms,
        }
    }

    /// Sondagem detalhada de porta TCP distinguindo `Open`, `Closed` (ConnectionRefused),
    /// `TimedOut`, `Unreachable` e outros erros.
    pub fn tcp_probe_detailed(
        host: &str,
        port: u16,
        timeout_secs: f64,
        grab_banner: bool,
        banner_max_bytes: usize,
    ) -> DetailedPortScanResult {
        let t0 = Instant::now();
        match TcpClient::connect_timeout((host, port), timeout_secs) {
            Ok(mut cli) => {
                let latency_ms = t0.elapsed().as_secs_f64() * 1000.0;
                let mut banner = None;
                if grab_banner && banner_max_bytes > 0 {
                    let _ = cli.set_timeout(Some(timeout_secs));
                    if let Ok(dados) = cli.recv_capped(banner_max_bytes, banner_max_bytes) {
                        if !dados.is_empty() {
                            let texto = String::from_utf8_lossy(&dados).trim().to_string();
                            if !texto.is_empty() {
                                banner = Some(texto);
                            }
                        }
                    }
                }
                let _ = cli.close();
                DetailedPortScanResult {
                    host: host.to_string(),
                    port,
                    state: PortState::Open,
                    banner,
                    latency_ms,
                    error_kind: None,
                }
            }
            Err(e) => {
                let kind = e.kind();
                let state = match kind {
                    io::ErrorKind::ConnectionRefused => PortState::Closed,
                    io::ErrorKind::TimedOut => PortState::TimedOut,
                    io::ErrorKind::NetworkUnreachable | io::ErrorKind::HostUnreachable => {
                        PortState::Unreachable
                    }
                    _ => PortState::Error(e.to_string()),
                };
                DetailedPortScanResult {
                    host: host.to_string(),
                    port,
                    state,
                    banner: None,
                    latency_ms: t0.elapsed().as_secs_f64() * 1000.0,
                    error_kind: Some(kind),
                }
            }
        }
    }

    /// Executa scan TCP concorrente (limitado a [`MAX_SCAN_THREADS`] = 256 workers).
    pub fn tcp_scan(
        host: &str,
        ports: &[u16],
        timeout_secs: f64,
        threads: usize,
        grab_banner: bool,
    ) -> Vec<PortScanResult> {
        let cfg = ScanConfig {
            timeout_secs,
            threads,
            grab_banner,
            banner_max_bytes: 256,
            rate_limit_per_sec: None,
        };
        tcp_scan_with_config(host, ports, &cfg)
            .into_iter()
            .map(|det| PortScanResult {
                host: det.host,
                port: det.port,
                open: det.state == PortState::Open,
                banner: det.banner,
                latency_ms: det.latency_ms,
            })
            .collect()
    }

    /// Executa scan TCP concorrente usando [`ScanConfig`] com controle de threads e rate limit.
    pub fn tcp_scan_with_config(
        host: &str,
        ports: &[u16],
        config: &ScanConfig,
    ) -> Vec<DetailedPortScanResult> {
        if ports.is_empty() {
            return Vec::new();
        }
        let num_workers = config.threads.clamp(1, MAX_SCAN_THREADS).min(ports.len());
        let limiter = config
            .rate_limit_per_sec
            .and_then(|r| RateLimiter::new(r).ok());
        let indice = AtomicUsize::new(0);
        let resultados: Mutex<Vec<DetailedPortScanResult>> =
            Mutex::new(Vec::with_capacity(ports.len()));

        std::thread::scope(|s| {
            for _ in 0..num_workers {
                s.spawn(|| {
                    let mut lote_local = Vec::new();
                    loop {
                        let idx = indice.fetch_add(1, Ordering::Relaxed);
                        if idx >= ports.len() {
                            break;
                        }
                        if let Some(ref lim) = limiter {
                            lim.acquire();
                        }
                        let porta = ports[idx];
                        lote_local.push(tcp_probe_detailed(
                            host,
                            porta,
                            config.timeout_secs,
                            config.grab_banner,
                            config.banner_max_bytes,
                        ));
                    }
                    if !lote_local.is_empty() {
                        if let Ok(mut guard) = resultados.lock() {
                            guard.extend(lote_local);
                        }
                    }
                });
            }
        });

        let mut final_vec = resultados.into_inner().unwrap_or_default();
        final_vec.sort_by_key(|r| r.port);
        final_vec
    }

    // ========================================================================
    // Packet Crafting (IPv4/TCP e IPv4/UDP) & Streaming PCAP
    // ========================================================================

    pub const TCP_FIN: u8 = 0x01;
    pub const TCP_SYN: u8 = 0x02;
    pub const TCP_RST: u8 = 0x04;
    pub const TCP_PSH: u8 = 0x08;
    pub const TCP_ACK: u8 = 0x10;
    pub const TCP_URG: u8 = 0x20;
    pub const TCP_ECE: u8 = 0x40;
    pub const TCP_CWR: u8 = 0x80;

    /// Erros de construção, validação ou leitura de pacotes e arquivos `.pcap`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum NetPacketError {
        PayloadTooLarge,
        InvalidHeader(String),
        ChecksumMismatch,
        InvalidTimestamp,
        PacketTooLargeForSnaplen,
        InvalidPcapFormat(String),
    }

    impl fmt::Display for NetPacketError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                NetPacketError::PayloadTooLarge => {
                    write!(f, "ValueError: payload excede o tamanho máximo do pacote IPv4")
                }
                NetPacketError::InvalidHeader(msg) => {
                    write!(f, "ValueError: cabeçalho inválido ({})", msg)
                }
                NetPacketError::ChecksumMismatch => {
                    write!(f, "ValueError: falha na validação do checksum")
                }
                NetPacketError::InvalidTimestamp => {
                    write!(f, "ValueError: ts_usec deve ser menor que 1_000_000")
                }
                NetPacketError::PacketTooLargeForSnaplen => {
                    write!(f, "ValueError: tamanho do pacote excede snaplen do PCAP")
                }
                NetPacketError::InvalidPcapFormat(msg) => {
                    write!(f, "ValueError: formato PCAP inválido ({})", msg)
                }
            }
        }
    }

    impl std::error::Error for NetPacketError {}

    /// Acumulador interno em `u64` para evitar overflow de soma em buffers grandes.
    #[inline]
    fn checksum_add_slice(mut soma: u64, dados: &[u8]) -> u64 {
        let mut chunks = dados.chunks_exact(2);
        for par in &mut chunks {
            soma += u16::from_be_bytes([par[0], par[1]]) as u64;
        }
        if let Some(&ultimo) = chunks.remainder().first() {
            soma += u16::from_be_bytes([ultimo, 0]) as u64;
        }
        soma
    }

    #[inline]
    fn checksum_finalize(mut soma: u64) -> u16 {
        while (soma >> 16) != 0 {
            soma = (soma & 0xFFFF) + (soma >> 16);
        }
        !(soma as u16)
    }

    /// Calcula o Internet Checksum padrão (RFC 1071) em complemento de um de 16 bits,
    /// utilizando acumulador `u64` para prevenir overflow em buffers grandes.
    pub fn rfc1071_checksum(dados: &[u8]) -> u16 {
        checksum_finalize(checksum_add_slice(0, dados))
    }

    /// Calcula o checksum de um segmento TCP/UDP incluindo o pseudo-cabeçalho IPv4 **sem
    /// alocação de buffer intermediário**.
    fn ipv4_pseudo_checksum(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, proto: u8, segment: &[u8]) -> u16 {
        let s = src_ip.octets();
        let d = dst_ip.octets();
        let mut soma: u64 = 0;
        soma += u16::from_be_bytes([s[0], s[1]]) as u64;
        soma += u16::from_be_bytes([s[2], s[3]]) as u64;
        soma += u16::from_be_bytes([d[0], d[1]]) as u64;
        soma += u16::from_be_bytes([d[2], d[3]]) as u64;
        soma += proto as u64;
        soma += (segment.len() as u16) as u64;
        soma = checksum_add_slice(soma, segment);
        checksum_finalize(soma)
    }

    /// Calcula o checksum TCP com pseudo-cabeçalho IPv4 (zero alocação).
    pub fn ipv4_tcp_checksum(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, tcp_segment: &[u8]) -> u16 {
        ipv4_pseudo_checksum(src_ip, dst_ip, 6, tcp_segment)
    }

    /// Versão falível de [`ipv4_tcp_checksum`] validando que `tcp_segment.len() <= 65515`.
    pub fn try_ipv4_tcp_checksum(
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        tcp_segment: &[u8],
    ) -> Result<u16, NetPacketError> {
        if tcp_segment.len() > (u16::MAX as usize - 20) {
            return Err(NetPacketError::PayloadTooLarge);
        }
        Ok(ipv4_pseudo_checksum(src_ip, dst_ip, 6, tcp_segment))
    }

    /// Calcula o checksum UDP com pseudo-cabeçalho IPv4 (zero alocação).
    pub fn ipv4_udp_checksum(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, udp_segment: &[u8]) -> u16 {
        ipv4_pseudo_checksum(src_ip, dst_ip, 17, udp_segment)
    }

    /// Construtor e dissecador de pacotes IPv4/TCP (`IP()/TCP()/Raw()`).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Ipv4TcpPacket {
        pub src_ip: Ipv4Addr,
        pub dst_ip: Ipv4Addr,
        pub src_port: u16,
        pub dst_port: u16,
        pub seq: u32,
        pub ack: u32,
        pub flags: u8,
        pub window: u16,
        pub ttl: u8,
        pub identification: u16,
        pub payload: Vec<u8>,
    }

    impl Ipv4TcpPacket {
        pub fn new(
            src_ip: Ipv4Addr,
            dst_ip: Ipv4Addr,
            src_port: u16,
            dst_port: u16,
            flags: u8,
        ) -> Self {
            Ipv4TcpPacket {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                seq: 1,
                ack: 0,
                flags,
                window: 64240,
                ttl: 64,
                identification: 0x1337,
                payload: Vec::new(),
            }
        }

        pub fn with_payload<D: AsRef<[u8]>>(mut self, payload: D) -> Self {
            self.payload = payload.as_ref().to_vec();
            self
        }

        pub fn with_seq(mut self, seq: u32) -> Self {
            self.seq = seq;
            self
        }

        pub fn with_ack(mut self, ack: u32) -> Self {
            self.ack = ack;
            self
        }

        pub fn with_ttl(mut self, ttl: u8) -> Self {
            self.ttl = ttl;
            self
        }

        /// Serializa validando que `40 + payload.len() <= 65535` (sem truncamento silencioso).
        pub fn try_build(&self) -> Result<Vec<u8>, NetPacketError> {
            let total_usize = 40usize
                .checked_add(self.payload.len())
                .ok_or(NetPacketError::PayloadTooLarge)?;
            if total_usize > u16::MAX as usize {
                return Err(NetPacketError::PayloadTooLarge);
            }
            let total_len = total_usize as u16;
            let mut pkt = Vec::with_capacity(total_usize);

            // Cabeçalho IPv4 (20 bytes)
            pkt.push(0x45);
            pkt.push(0x00);
            pkt.extend_from_slice(&total_len.to_be_bytes());
            pkt.extend_from_slice(&self.identification.to_be_bytes());
            pkt.extend_from_slice(&0x4000u16.to_be_bytes());
            pkt.push(self.ttl);
            pkt.push(6);
            pkt.extend_from_slice(&[0u8, 0u8]);
            pkt.extend_from_slice(&self.src_ip.octets());
            pkt.extend_from_slice(&self.dst_ip.octets());

            let ip_csum = rfc1071_checksum(&pkt[..20]);
            pkt[10..12].copy_from_slice(&ip_csum.to_be_bytes());

            // Cabeçalho TCP (20 bytes) + payload
            pkt.extend_from_slice(&self.src_port.to_be_bytes());
            pkt.extend_from_slice(&self.dst_port.to_be_bytes());
            pkt.extend_from_slice(&self.seq.to_be_bytes());
            pkt.extend_from_slice(&self.ack.to_be_bytes());
            pkt.push(0x50);
            pkt.push(self.flags);
            pkt.extend_from_slice(&self.window.to_be_bytes());
            pkt.extend_from_slice(&[0u8, 0u8]);
            pkt.extend_from_slice(&[0u8, 0u8]);
            pkt.extend_from_slice(&self.payload);

            let tcp_csum = ipv4_tcp_checksum(self.src_ip, self.dst_ip, &pkt[20..]);
            pkt[36..38].copy_from_slice(&tcp_csum.to_be_bytes());
            Ok(pkt)
        }

        /// Serializa o pacote completo IPv4/TCP.
        ///
        /// # Panics
        /// Causa panic se `payload.len() > 65495`. Para retorno falível, use [`try_build`](Self::try_build).
        pub fn build(&self) -> Vec<u8> {
            self.try_build()
                .expect("ValueError: payload excede 65495 bytes em Ipv4TcpPacket::build")
        }

        /// Disseca um pacote IPv4/TCP bruto validando `total_len` (descarta eventuais bytes
        /// residuais de padding Ethernet).
        pub fn parse(raw: &[u8]) -> Option<Self> {
            if raw.len() < 40 || (raw[0] >> 4) != 4 {
                return None;
            }
            let ihl = ((raw[0] & 0x0F) as usize) * 4;
            let total_len = u16::from_be_bytes([raw[2], raw[3]]) as usize;
            if ihl < 20 || total_len < ihl + 20 || total_len > raw.len() || raw[9] != 6 {
                return None;
            }
            let pkt = &raw[..total_len];
            let identification = u16::from_be_bytes([pkt[4], pkt[5]]);
            let ttl = pkt[8];
            let src_ip = Ipv4Addr::new(pkt[12], pkt[13], pkt[14], pkt[15]);
            let dst_ip = Ipv4Addr::new(pkt[16], pkt[17], pkt[18], pkt[19]);

            let tcp = &pkt[ihl..];
            let src_port = u16::from_be_bytes([tcp[0], tcp[1]]);
            let dst_port = u16::from_be_bytes([tcp[2], tcp[3]]);
            let seq = u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]);
            let ack = u32::from_be_bytes([tcp[8], tcp[9], tcp[10], tcp[11]]);
            let data_offset = ((tcp[12] >> 4) as usize) * 4;
            if data_offset < 20 || tcp.len() < data_offset {
                return None;
            }
            let flags = tcp[13];
            let window = u16::from_be_bytes([tcp[14], tcp[15]]);
            let payload = tcp[data_offset..].to_vec();

            Some(Ipv4TcpPacket {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                seq,
                ack,
                flags,
                window,
                ttl,
                identification,
                payload,
            })
        }

        /// Disseca o pacote validando também ausência de fragmentação IP e integridade dos
        /// checksums IPv4 e TCP.
        pub fn parse_checked(raw: &[u8]) -> Result<Self, NetPacketError> {
            if raw.len() < 40 || (raw[0] >> 4) != 4 {
                return Err(NetPacketError::InvalidHeader(
                    "tamanho insuficiente ou versão diferente de IPv4".to_string(),
                ));
            }
            let ihl = ((raw[0] & 0x0F) as usize) * 4;
            let total_len = u16::from_be_bytes([raw[2], raw[3]]) as usize;
            if ihl < 20 || total_len < ihl + 20 || total_len > raw.len() {
                return Err(NetPacketError::InvalidHeader(
                    "IHL ou total_len inconsistente".to_string(),
                ));
            }
            if raw[9] != 6 {
                return Err(NetPacketError::InvalidHeader(
                    "protocolo IP diferente de TCP (6)".to_string(),
                ));
            }
            let flags_frag = u16::from_be_bytes([raw[6], raw[7]]);
            if (flags_frag & 0x3FFF) != 0 {
                return Err(NetPacketError::InvalidHeader(
                    "pacote IPv4 fragmentado não suportado em parse_checked".to_string(),
                ));
            }
            let pkt = &raw[..total_len];
            if rfc1071_checksum(&pkt[..ihl]) != 0 {
                return Err(NetPacketError::ChecksumMismatch);
            }
            let src_ip = Ipv4Addr::new(pkt[12], pkt[13], pkt[14], pkt[15]);
            let dst_ip = Ipv4Addr::new(pkt[16], pkt[17], pkt[18], pkt[19]);
            let tcp = &pkt[ihl..];
            if ipv4_tcp_checksum(src_ip, dst_ip, tcp) != 0 {
                return Err(NetPacketError::ChecksumMismatch);
            }
            Self::parse(pkt).ok_or_else(|| {
                NetPacketError::InvalidHeader("data_offset TCP inválido".to_string())
            })
        }
    }

    /// Construtor e dissecador de pacotes IPv4/UDP.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Ipv4UdpPacket {
        pub src_ip: Ipv4Addr,
        pub dst_ip: Ipv4Addr,
        pub src_port: u16,
        pub dst_port: u16,
        pub ttl: u8,
        pub identification: u16,
        pub payload: Vec<u8>,
    }

    impl Ipv4UdpPacket {
        pub fn new(
            src_ip: Ipv4Addr,
            dst_ip: Ipv4Addr,
            src_port: u16,
            dst_port: u16,
        ) -> Self {
            Ipv4UdpPacket {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                ttl: 64,
                identification: 0x1337,
                payload: Vec::new(),
            }
        }

        pub fn with_payload<D: AsRef<[u8]>>(mut self, payload: D) -> Self {
            self.payload = payload.as_ref().to_vec();
            self
        }

        pub fn with_ttl(mut self, ttl: u8) -> Self {
            self.ttl = ttl;
            self
        }

        pub fn try_build(&self) -> Result<Vec<u8>, NetPacketError> {
            let total_usize = 28usize
                .checked_add(self.payload.len())
                .ok_or(NetPacketError::PayloadTooLarge)?;
            if total_usize > u16::MAX as usize {
                return Err(NetPacketError::PayloadTooLarge);
            }
            let total_len = total_usize as u16;
            let udp_len = (8 + self.payload.len()) as u16;
            let mut pkt = Vec::with_capacity(total_usize);

            pkt.push(0x45);
            pkt.push(0x00);
            pkt.extend_from_slice(&total_len.to_be_bytes());
            pkt.extend_from_slice(&self.identification.to_be_bytes());
            pkt.extend_from_slice(&0x4000u16.to_be_bytes());
            pkt.push(self.ttl);
            pkt.push(17);
            pkt.extend_from_slice(&[0u8, 0u8]);
            pkt.extend_from_slice(&self.src_ip.octets());
            pkt.extend_from_slice(&self.dst_ip.octets());
            let ip_csum = rfc1071_checksum(&pkt[..20]);
            pkt[10..12].copy_from_slice(&ip_csum.to_be_bytes());

            pkt.extend_from_slice(&self.src_port.to_be_bytes());
            pkt.extend_from_slice(&self.dst_port.to_be_bytes());
            pkt.extend_from_slice(&udp_len.to_be_bytes());
            pkt.extend_from_slice(&[0u8, 0u8]);
            pkt.extend_from_slice(&self.payload);

            let mut udp_csum = ipv4_udp_checksum(self.src_ip, self.dst_ip, &pkt[20..]);
            if udp_csum == 0 {
                udp_csum = 0xFFFF;
            }
            pkt[26..28].copy_from_slice(&udp_csum.to_be_bytes());
            Ok(pkt)
        }

        pub fn build(&self) -> Vec<u8> {
            self.try_build()
                .expect("ValueError: payload excede 65507 bytes em Ipv4UdpPacket::build")
        }

        pub fn parse(raw: &[u8]) -> Option<Self> {
            if raw.len() < 28 || (raw[0] >> 4) != 4 {
                return None;
            }
            let ihl = ((raw[0] & 0x0F) as usize) * 4;
            let total_len = u16::from_be_bytes([raw[2], raw[3]]) as usize;
            if ihl < 20 || total_len < ihl + 8 || total_len > raw.len() || raw[9] != 17 {
                return None;
            }
            let pkt = &raw[..total_len];
            let identification = u16::from_be_bytes([pkt[4], pkt[5]]);
            let ttl = pkt[8];
            let src_ip = Ipv4Addr::new(pkt[12], pkt[13], pkt[14], pkt[15]);
            let dst_ip = Ipv4Addr::new(pkt[16], pkt[17], pkt[18], pkt[19]);

            let udp = &pkt[ihl..];
            let udp_len = u16::from_be_bytes([udp[4], udp[5]]) as usize;
            if udp_len < 8 || udp_len > udp.len() {
                return None;
            }
            let src_port = u16::from_be_bytes([udp[0], udp[1]]);
            let dst_port = u16::from_be_bytes([udp[2], udp[3]]);
            let payload = udp[8..udp_len].to_vec();

            Some(Ipv4UdpPacket {
                src_ip,
                dst_ip,
                src_port,
                dst_port,
                ttl,
                identification,
                payload,
            })
        }

        pub fn parse_checked(raw: &[u8]) -> Result<Self, NetPacketError> {
            if raw.len() < 28 || (raw[0] >> 4) != 4 {
                return Err(NetPacketError::InvalidHeader(
                    "tamanho insuficiente ou versão diferente de IPv4".to_string(),
                ));
            }
            let ihl = ((raw[0] & 0x0F) as usize) * 4;
            let total_len = u16::from_be_bytes([raw[2], raw[3]]) as usize;
            if ihl < 20 || total_len < ihl + 8 || total_len > raw.len() || raw[9] != 17 {
                return Err(NetPacketError::InvalidHeader(
                    "cabeçalho IPv4/UDP inválido".to_string(),
                ));
            }
            let pkt = &raw[..total_len];
            if rfc1071_checksum(&pkt[..ihl]) != 0 {
                return Err(NetPacketError::ChecksumMismatch);
            }
            let src_ip = Ipv4Addr::new(pkt[12], pkt[13], pkt[14], pkt[15]);
            let dst_ip = Ipv4Addr::new(pkt[16], pkt[17], pkt[18], pkt[19]);
            let udp = &pkt[ihl..];
            let stored_csum = u16::from_be_bytes([udp[6], udp[7]]);
            if stored_csum != 0 && ipv4_udp_checksum(src_ip, dst_ip, udp) != 0 {
                return Err(NetPacketError::ChecksumMismatch);
            }
            Self::parse(pkt).ok_or_else(|| {
                NetPacketError::InvalidHeader("comprimento UDP inválido".to_string())
            })
        }
    }

    fn build_pcap_global_header(linktype: u32, snaplen: u32) -> [u8; 24] {
        let mut h = [0u8; 24];
        h[0..4].copy_from_slice(&0xA1B2_C3D4u32.to_le_bytes());
        h[4..6].copy_from_slice(&2u16.to_le_bytes());
        h[6..8].copy_from_slice(&4u16.to_le_bytes());
        h[8..12].copy_from_slice(&0i32.to_le_bytes());
        h[12..16].copy_from_slice(&0u32.to_le_bytes());
        h[16..20].copy_from_slice(&snaplen.to_le_bytes());
        h[20..24].copy_from_slice(&linktype.to_le_bytes());
        h
    }

    /// Gerador em memória de arquivos `.pcap` (Libpcap v2.4).
    /// Para gravação de arquivos grandes sem acumular tudo em RAM, use [`StreamingPcapWriter`].
    #[derive(Debug, Clone)]
    pub struct PcapWriter {
        buf: Vec<u8>,
        linktype: u32,
        snaplen: u32,
    }

    impl PcapWriter {
        pub fn new_raw_ip() -> Self {
            Self::with_linktype(101)
        }

        pub fn new_ethernet() -> Self {
            Self::with_linktype(1)
        }

        /// Inicializa um `PcapWriter` com `linktype` e `snaplen` explícitos.
        pub fn with_snaplen(linktype: u32, snaplen: u32) -> Self {
            let mut buf = Vec::with_capacity(256);
            buf.extend_from_slice(&build_pcap_global_header(linktype, snaplen));
            PcapWriter {
                buf,
                linktype,
                snaplen,
            }
        }

        fn with_linktype(linktype: u32) -> Self {
            Self::with_snaplen(linktype, 65535)
        }

        /// Adiciona pacote validando `ts_usec < 1_000_000` e `dados.len() <= snaplen`.
        pub fn try_add_packet(
            &mut self,
            ts_sec: u32,
            ts_usec: u32,
            dados: &[u8],
        ) -> Result<(), NetPacketError> {
            if ts_usec >= 1_000_000 {
                return Err(NetPacketError::InvalidTimestamp);
            }
            if dados.len() > self.snaplen as usize {
                return Err(NetPacketError::PacketTooLargeForSnaplen);
            }
            let tam = dados.len() as u32;
            self.buf.extend_from_slice(&ts_sec.to_le_bytes());
            self.buf.extend_from_slice(&ts_usec.to_le_bytes());
            self.buf.extend_from_slice(&tam.to_le_bytes());
            self.buf.extend_from_slice(&tam.to_le_bytes());
            self.buf.extend_from_slice(dados);
            Ok(())
        }

        pub fn add_packet(&mut self, ts_sec: u32, ts_usec: u32, dados: &[u8]) {
            let tam = dados.len() as u32;
            self.buf.extend_from_slice(&ts_sec.to_le_bytes());
            self.buf.extend_from_slice(&ts_usec.to_le_bytes());
            self.buf.extend_from_slice(&tam.to_le_bytes());
            self.buf.extend_from_slice(&tam.to_le_bytes());
            self.buf.extend_from_slice(dados);
        }

        pub fn add_packet_now(&mut self, dados: &[u8]) {
            let d = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            self.add_packet(d.as_secs() as u32, d.subsec_micros(), dados);
        }

        pub fn add_ipv4_as_ethernet(
            &mut self,
            ts_sec: u32,
            ts_usec: u32,
            src_mac: [u8; 6],
            dst_mac: [u8; 6],
            ipv4_packet: &[u8],
        ) {
            let mut frame = Vec::with_capacity(14 + ipv4_packet.len());
            frame.extend_from_slice(&dst_mac);
            frame.extend_from_slice(&src_mac);
            frame.extend_from_slice(&0x0800u16.to_be_bytes());
            frame.extend_from_slice(ipv4_packet);
            self.add_packet(ts_sec, ts_usec, &frame);
        }

        #[inline]
        pub fn as_bytes(&self) -> &[u8] {
            &self.buf
        }

        #[inline]
        pub fn into_bytes(self) -> Vec<u8> {
            self.buf
        }

        pub fn save<P: AsRef<Path>>(&self, caminho: P) -> io::Result<()> {
            fs::write(caminho, &self.buf)
        }
    }

    /// Escritor `.pcap` em streaming (`BufWriter<W>`), gravando cabeçalho e pacotes diretamente no
    /// fluxo de saída com consumo constante de RAM.
    #[derive(Debug)]
    pub struct StreamingPcapWriter<W: Write> {
        writer: BufWriter<W>,
        snaplen: u32,
    }

    impl<W: Write> StreamingPcapWriter<W> {
        pub fn new_raw_ip(writer: W) -> io::Result<Self> {
            Self::with_linktype_and_snaplen(writer, 101, 65535)
        }

        pub fn new_ethernet(writer: W) -> io::Result<Self> {
            Self::with_linktype_and_snaplen(writer, 1, 65535)
        }

        pub fn with_linktype_and_snaplen(
            writer: W,
            linktype: u32,
            snaplen: u32,
        ) -> io::Result<Self> {
            let mut bw = BufWriter::new(writer);
            bw.write_all(&build_pcap_global_header(linktype, snaplen))?;
            Ok(StreamingPcapWriter {
                writer: bw,
                snaplen,
            })
        }

        pub fn write_packet(
            &mut self,
            ts_sec: u32,
            ts_usec: u32,
            dados: &[u8],
        ) -> io::Result<()> {
            if ts_usec >= 1_000_000 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "ts_usec deve ser < 1_000_000",
                ));
            }
            if dados.len() > self.snaplen as usize {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "pacote excede snaplen do PCAP",
                ));
            }
            let tam = dados.len() as u32;
            self.writer.write_all(&ts_sec.to_le_bytes())?;
            self.writer.write_all(&ts_usec.to_le_bytes())?;
            self.writer.write_all(&tam.to_le_bytes())?;
            self.writer.write_all(&tam.to_le_bytes())?;
            self.writer.write_all(dados)
        }

        pub fn write_packet_now(&mut self, dados: &[u8]) -> io::Result<()> {
            let d = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            self.write_packet(d.as_secs() as u32, d.subsec_micros(), dados)
        }

        #[inline]
        pub fn flush(&mut self) -> io::Result<()> {
            self.writer.flush()
        }

        #[inline]
        pub fn into_inner(self) -> Result<W, io::IntoInnerError<BufWriter<W>>> {
            self.writer.into_inner()
        }
    }

    impl StreamingPcapWriter<File> {
        pub fn create_raw_ip<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            Self::new_raw_ip(File::create(path)?)
        }

        pub fn create_ethernet<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            Self::new_ethernet(File::create(path)?)
        }
    }

    /// Registro de pacote lido de um arquivo `.pcap` por [`PcapReader`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PcapPacket {
        pub ts_sec: u32,
        pub ts_usec: u32,
        pub orig_len: u32,
        pub data: Vec<u8>,
    }

    /// Leitor defensivo em streaming de arquivos `.pcap` (Libpcap v2.4, LE/BE).
    #[derive(Debug)]
    pub struct PcapReader<R: Read> {
        reader: BufReader<R>,
        big_endian: bool,
        snaplen: u32,
        linktype: u32,
    }

    impl<R: Read> PcapReader<R> {
        pub fn new(reader: R) -> io::Result<Self> {
            let mut br = BufReader::new(reader);
            let mut hdr = [0u8; 24];
            br.read_exact(&mut hdr)?;
            let magic_le = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
            let big_endian = match magic_le {
                0xA1B2_C3D4 | 0xA1B2_3C4D => false,
                0xD4C3_B2A1 | 0x4D3C_B2A1 => true,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "magic number PCAP inválido",
                    ));
                }
            };
            let rd16 = |b: &[u8]| -> u16 {
                if big_endian {
                    u16::from_be_bytes([b[0], b[1]])
                } else {
                    u16::from_le_bytes([b[0], b[1]])
                }
            };
            let rd32 = |b: &[u8]| -> u32 {
                if big_endian {
                    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
                } else {
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                }
            };
            let major = rd16(&hdr[4..6]);
            let minor = rd16(&hdr[6..8]);
            if major != 2 || minor != 4 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("versão PCAP não suportada ({}.{})", major, minor),
                ));
            }
            let snaplen = rd32(&hdr[16..20]);
            let linktype = rd32(&hdr[20..24]);
            Ok(PcapReader {
                reader: br,
                big_endian,
                snaplen,
                linktype,
            })
        }

        #[inline]
        pub const fn linktype(&self) -> u32 {
            self.linktype
        }

        #[inline]
        pub const fn snaplen(&self) -> u32 {
            self.snaplen
        }

        /// Lê o próximo pacote validando o tamanho máximo `max_packet_bytes` antes de alocar
        /// (proteção contra cabeçalhos `.pcap` malformados com `incl_len` gigantesco).
        pub fn next_packet_limited(
            &mut self,
            max_packet_bytes: usize,
        ) -> io::Result<Option<PcapPacket>> {
            let mut hdr = [0u8; 16];
            let mut lidos = 0usize;
            while lidos < 16 {
                match self.reader.read(&mut hdr[lidos..]) {
                    Ok(0) => {
                        if lidos == 0 {
                            return Ok(None);
                        }
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "cabeçalho de pacote PCAP truncado",
                        ));
                    }
                    Ok(n) => lidos += n,
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }

            let rd32 = |b: &[u8]| -> u32 {
                if self.big_endian {
                    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
                } else {
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                }
            };
            let ts_sec = rd32(&hdr[0..4]);
            let ts_usec = rd32(&hdr[4..8]);
            let incl_len = rd32(&hdr[8..12]);
            let orig_len = rd32(&hdr[12..16]);

            if incl_len as usize > max_packet_bytes {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "incl_len ({}) excede limite máximo ({})",
                        incl_len, max_packet_bytes
                    ),
                ));
            }
            let mut data = vec![0u8; incl_len as usize];
            self.reader.read_exact(&mut data)?;
            Ok(Some(PcapPacket {
                ts_sec,
                ts_usec,
                orig_len,
                data,
            }))
        }
    }

    impl PcapReader<File> {
        pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            Self::new(File::open(path)?)
        }
    }

    // ========================================================================
    // Living off the Land — Leitura direta de /proc/net/tcp e /proc/net/arp (Linux)
    // ========================================================================

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProcTcpEntry {
        pub sl: usize,
        pub local_addr: SocketAddrV4,
        pub rem_addr: SocketAddrV4,
        pub state: &'static str,
        pub uid: u32,
        pub inode: u64,
    }

    fn estado_tcp_linux(codigo: u8) -> &'static str {
        match codigo {
            0x01 => "ESTABLISHED",
            0x02 => "SYN_SENT",
            0x03 => "SYN_RECV",
            0x04 => "FIN_WAIT1",
            0x05 => "FIN_WAIT2",
            0x06 => "TIME_WAIT",
            0x07 => "CLOSE",
            0x08 => "CLOSE_WAIT",
            0x09 => "LAST_ACK",
            0x0A => "LISTEN",
            0x0B => "CLOSING",
            0x0C => "NEW_SYN_RECV",
            _ => "UNKNOWN",
        }
    }

    fn parse_hex_ip_port_le(campo: &str) -> Option<SocketAddrV4> {
        let (ip_hex, port_hex) = campo.split_once(':')?;
        let ip_le = u32::from_str_radix(ip_hex, 16).ok()?;
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        let octetos = ip_le.to_le_bytes();
        Some(SocketAddrV4::new(
            Ipv4Addr::new(octetos[0], octetos[1], octetos[2], octetos[3]),
            port,
        ))
    }

    pub fn parse_proc_net_tcp_str(conteudo: &str) -> Vec<ProcTcpEntry> {
        let mut entradas = Vec::new();
        for (i, linha) in conteudo.lines().enumerate() {
            if i == 0 {
                continue;
            }
            let cols: Vec<&str> = linha.split_whitespace().collect();
            if cols.len() < 10 {
                continue;
            }
            let sl = cols[0].trim_end_matches(':').parse::<usize>().unwrap_or(0);
            let local_addr = match parse_hex_ip_port_le(cols[1]) {
                Some(a) => a,
                None => continue,
            };
            let rem_addr = match parse_hex_ip_port_le(cols[2]) {
                Some(a) => a,
                None => continue,
            };
            let state_code = u8::from_str_radix(cols[3], 16).unwrap_or(0);
            let uid = cols[7].parse::<u32>().unwrap_or(0);
            let inode = cols[9].parse::<u64>().unwrap_or(0);

            entradas.push(ProcTcpEntry {
                sl,
                local_addr,
                rem_addr,
                state: estado_tcp_linux(state_code),
                uid,
                inode,
            });
        }
        entradas
    }

    /// Lê diretamente `/proc/net/tcp` (específico de Linux; falha com `io::Error` em outros SOs).
    pub fn proc_net_tcp() -> io::Result<Vec<ProcTcpEntry>> {
        let conteudo = fs::read_to_string("/proc/net/tcp")?;
        Ok(parse_proc_net_tcp_str(&conteudo))
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ProcArpEntry {
        pub ip: Ipv4Addr,
        pub hw_type: String,
        pub flags: String,
        pub mac: String,
        pub mask: String,
        pub device: String,
    }

    pub fn parse_proc_net_arp_str(conteudo: &str) -> Vec<ProcArpEntry> {
        let mut entradas = Vec::new();
        for (i, linha) in conteudo.lines().enumerate() {
            if i == 0 {
                continue;
            }
            let cols: Vec<&str> = linha.split_whitespace().collect();
            if cols.len() < 6 {
                continue;
            }
            if let Ok(ip) = cols[0].parse::<Ipv4Addr>() {
                entradas.push(ProcArpEntry {
                    ip,
                    hw_type: cols[1].to_string(),
                    flags: cols[2].to_string(),
                    mac: cols[3].to_string(),
                    mask: cols[4].to_string(),
                    device: cols[5].to_string(),
                });
            }
        }
        entradas
    }

    /// Lê diretamente `/proc/net/arp` (específico de Linux; falha com `io::Error` em outros SOs).
    pub fn proc_net_arp() -> io::Result<Vec<ProcArpEntry>> {
        let conteudo = fs::read_to_string("/proc/net/arp")?;
        Ok(parse_proc_net_arp_str(&conteudo))
    }
}

// ============================================================================
// 11) Números Aleatórios — PRNG determinístico/sem viés vs CSPRNG do SO
// ============================================================================
pub mod random_utils {
    //! Equivalente ao módulo `random` do Python em `std` pura, com separação explícita
    //! entre aleatoriedade determinística (PRNG) e entropia criptográfica do sistema operacional.
    //!
    //! - **PRNG Global** ([`seed`], [`random`], [`randint`], [`try_randint`], [`choice`],
    //!   [`shuffle`], [`uniform`], [`randbytes_insecure`]): utiliza **SplitMix64** protegido por
    //!   `Mutex` e remoção de viés de módulo por *rejection sampling*.
    //! - **Instâncias independentes** ([`PyRng`]): gerador **Xoshiro256\*\*** inicializado via
    //!   `SplitMix64`, ideal para simulações e testes reproduzíveis sem estado global compartilhado.
    //! - **Aviso de segurança:** nem `SplitMix64` nem `Xoshiro256**` são CSPRNGs. **Nunca** os
    //!   utilize para gerar chaves criptográficas, tokens de sessão, nonces ou senhas. Para
    //!   bytes aleatórios criptográficos, utilize [`os_random_bytes`], que consulta `/dev/urandom`
    //!   no Unix e falha explicitamente com `ErrorKind::Unsupported` onde não há fonte acessível
    //!   via `std` (sem *fallback* inseguro para PRNG).

    use std::io::{self, Read};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ESTADO_GLOBAL: Mutex<Option<u64>> = Mutex::new(None);

    fn semente_automatica() -> u64 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let pid = std::process::id() as u64;
        let end_pilha = &nanos as *const u64 as usize as u64;
        let mut z = nanos ^ pid.rotate_left(17) ^ end_pilha.rotate_right(11);
        z = splitmix64_mix(z);
        if z == 0 {
            0xCAFE_BABE_DEAD_BEEF
        } else {
            z
        }
    }

    #[inline]
    const fn splitmix64_mix(mut z: u64) -> u64 {
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_global_u64() -> u64 {
        let mut guard = match ESTADO_GLOBAL.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let state = guard.get_or_insert_with(semente_automatica);
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        splitmix64_mix(*state)
    }

    /// Gera um inteiro uniforme sem viés de módulo em `[0, bound - 1]` usando *rejection sampling*.
    /// Requer `bound >= 1` e `bound <= 2^64`.
    fn next_unbiased_u128<F: FnMut() -> u64>(mut gen_u64: F, bound: u128) -> u128 {
        debug_assert!(bound >= 1);
        if bound == (1u128 << 64) {
            return gen_u64() as u128;
        }
        let b = bound as u64;
        if b <= 1 {
            return 0;
        }
        // Limite equivalente a (2^64) % b sem precisar de u128 na divisão do laço.
        let threshold = b.wrapping_neg() % b;
        loop {
            let r = gen_u64();
            if r >= threshold {
                return (r % b) as u128;
            }
        }
    }

    /// `random.seed(valor)` — fixa a semente do gerador global **não criptográfico** (SplitMix64).
    pub fn seed(valor: u64) {
        let mut guard = match ESTADO_GLOBAL.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = Some(valor);
    }

    /// `random.random()` — devolve `f64` uniforme em `[0.0, 1.0)` (53 bits de precisão).
    #[inline]
    pub fn random() -> f64 {
        ((next_global_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// `random.uniform(a, b)` — devolve `f64` no intervalo `[a, b]` (ou `[b, a]` se `a > b`).
    #[inline]
    pub fn uniform(a: f64, b: f64) -> f64 {
        a + (b - a) * random()
    }

    /// `random.randint(low, high)` — inteiro uniforme sem viés no intervalo inclusivo
    /// `[low, high]`. Suporta todo o intervalo `(i64::MIN, i64::MAX)` sem overflow.
    ///
    /// # Panics
    /// Causa panic se `low > high`. Para versão falível sem panic, use [`try_randint`].
    pub fn randint(low: i64, high: i64) -> i64 {
        assert!(
            low <= high,
            "ValueError: intervalo vazio em randint({}, {})",
            low,
            high
        );
        let largura = (high as i128 - low as i128 + 1) as u128;
        let sorteado = next_unbiased_u128(next_global_u64, largura);
        (low as i128 + sorteado as i128) as i64
    }

    /// Versão segura de [`randint`] que devolve `Err` caso `low > high`.
    pub fn try_randint(low: i64, high: i64) -> Result<i64, String> {
        if low > high {
            return Err(format!(
                "ValueError: intervalo vazio em try_randint({}, {})",
                low, high
            ));
        }
        let largura = (high as i128 - low as i128 + 1) as u128;
        let sorteado = next_unbiased_u128(next_global_u64, largura);
        Ok((low as i128 + sorteado as i128) as i64)
    }

    /// `random.choice(seq)` — escolhe um elemento aleatório sem viés de módulo (`None` se vazia).
    #[inline]
    pub fn choice<T>(fatia: &[T]) -> Option<&T> {
        if fatia.is_empty() {
            None
        } else {
            let idx = next_unbiased_u128(next_global_u64, fatia.len() as u128) as usize;
            Some(&fatia[idx])
        }
    }

    /// `random.shuffle(seq)` — embaralha in-place via Fisher-Yates sem viés de módulo em $O(N)$.
    pub fn shuffle<T>(fatia: &mut [T]) {
        let n = fatia.len();
        for i in (1..n).rev() {
            let j = next_unbiased_u128(next_global_u64, (i + 1) as u128) as usize;
            fatia.swap(i, j);
        }
    }

    /// Gera `len` bytes pseudoaleatórios **NÃO CRIPTOGRÁFICOS** (para simulações e fuzzing).
    /// Para chaves/tokens/senhas, utilize [`os_random_bytes`].
    pub fn randbytes_insecure(len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let w = next_global_u64().to_le_bytes();
            let falta = len - out.len();
            out.extend_from_slice(&w[..falta.min(8)]);
        }
        out
    }

    /// Lê `len` bytes criptograficamente aleatórios do sistema operacional (`/dev/urandom` em Unix).
    ///
    /// **Transparência sobre portabilidade (`std`):**
    /// Em plataformas não-Unix (ex.: Windows sem FFI `BCryptGenRandom` ou WASM sem WASI), a `std`
    /// estável não expõe diretamente o CSPRNG do sistema. Nesses ambientes esta função retorna
    /// `Err(ErrorKind::Unsupported)` em vez de recorrer a um PRNG inseguro.
    pub fn os_random_bytes(len: usize) -> io::Result<Vec<u8>> {
        #[cfg(unix)]
        {
            let mut f = std::fs::File::open("/dev/urandom")?;
            let mut buf = vec![0u8; len];
            f.read_exact(&mut buf)?;
            Ok(buf)
        }
        #[cfg(not(unix))]
        {
            let _ = len;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "os_random_bytes requer fonte CSPRNG do SO (ex.: /dev/urandom no Unix) indisponível nesta plataforma usando apenas std",
            ))
        }
    }

    /// Gerador pseudoaleatório independente e reproduzível (**Xoshiro256\*\*** inicializado
    /// com **SplitMix64**). **Não é um CSPRNG.**
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PyRng {
        s: [u64; 4],
    }

    impl PyRng {
        /// Cria um gerador determinístico a partir de uma semente `u64`.
        pub fn from_seed(seed: u64) -> Self {
            let mut z = seed;
            let mut next_sm = || {
                z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
                splitmix64_mix(z)
            };
            let mut s = [next_sm(), next_sm(), next_sm(), next_sm()];
            if s == [0, 0, 0, 0] {
                s[0] = 0x9E37_79B9_7F4A_7C15;
            }
            PyRng { s }
        }

        /// Cria um gerador inicializado a partir de entropia do SO (`/dev/urandom` em Unix)
        /// ou fallback de alta resolução caso indisponível.
        pub fn from_entropy() -> Self {
            if let Ok(bytes) = os_random_bytes(32) {
                let mut s = [0u64; 4];
                for i in 0..4 {
                    let off = i * 8;
                    s[i] = u64::from_le_bytes([
                        bytes[off],
                        bytes[off + 1],
                        bytes[off + 2],
                        bytes[off + 3],
                        bytes[off + 4],
                        bytes[off + 5],
                        bytes[off + 6],
                        bytes[off + 7],
                    ]);
                }
                if s != [0, 0, 0, 0] {
                    return PyRng { s };
                }
            }
            Self::from_seed(semente_automatica())
        }

        /// Avança o estado Xoshiro256** e devolve o próximo `u64`.
        #[inline]
        pub fn next_u64(&mut self) -> u64 {
            let result = (self.s[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
            let t = self.s[1] << 17;
            self.s[2] ^= self.s[0];
            self.s[3] ^= self.s[1];
            self.s[1] ^= self.s[2];
            self.s[0] ^= self.s[3];
            self.s[2] ^= t;
            self.s[3] = self.s[3].rotate_left(45);
            result
        }

        /// `f64` uniforme em `[0.0, 1.0)`.
        #[inline]
        pub fn random(&mut self) -> f64 {
            ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
        }

        /// `f64` uniforme em `[a, b]`.
        #[inline]
        pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
            a + (b - a) * self.random()
        }

        /// Inteiro uniforme sem viés no intervalo fechado `[low, high]`.
        ///
        /// # Panics
        /// Causa panic se `low > high`.
        pub fn randint(&mut self, low: i64, high: i64) -> i64 {
            assert!(
                low <= high,
                "ValueError: intervalo vazio em PyRng::randint({}, {})",
                low,
                high
            );
            let largura = (high as i128 - low as i128 + 1) as u128;
            let sorteado = next_unbiased_u128(|| self.next_u64(), largura);
            (low as i128 + sorteado as i128) as i64
        }

        /// Versão segura sem panic de [`PyRng::randint`].
        pub fn try_randint(&mut self, low: i64, high: i64) -> Result<i64, String> {
            if low > high {
                return Err(format!(
                    "ValueError: intervalo vazio em PyRng::try_randint({}, {})",
                    low, high
                ));
            }
            let largura = (high as i128 - low as i128 + 1) as u128;
            let sorteado = next_unbiased_u128(|| self.next_u64(), largura);
            Ok((low as i128 + sorteado as i128) as i64)
        }

        /// Escolhe um item aleatório da fatia sem viés de módulo.
        pub fn choice<'a, T>(&mut self, fatia: &'a [T]) -> Option<&'a T> {
            if fatia.is_empty() {
                None
            } else {
                let idx = next_unbiased_u128(|| self.next_u64(), fatia.len() as u128) as usize;
                Some(&fatia[idx])
            }
        }

        /// Embaralha in-place sem viés de módulo (Fisher-Yates).
        pub fn shuffle<T>(&mut self, fatia: &mut [T]) {
            let n = fatia.len();
            for i in (1..n).rev() {
                let j = next_unbiased_u128(|| self.next_u64(), (i + 1) as u128) as usize;
                fatia.swap(i, j);
            }
        }

        /// `random.sample(population, k)` — seleciona `k` elementos distintos sem reposição.
        pub fn sample<'a, T>(&mut self, fatia: &'a [T], k: usize) -> Result<Vec<&'a T>, String> {
            if k > fatia.len() {
                return Err(format!(
                    "ValueError: sample maior que a população ({} > {})",
                    k,
                    fatia.len()
                ));
            }
            let mut indices: Vec<usize> = (0..fatia.len()).collect();
            let mut out = Vec::with_capacity(k);
            for i in 0..k {
                let rem = fatia.len() - i;
                let j = i + (next_unbiased_u128(|| self.next_u64(), rem as u128) as usize);
                indices.swap(i, j);
                out.push(&fatia[indices[i]]);
            }
            Ok(out)
        }

        /// Gera `len` bytes pseudoaleatórios não criptográficos.
        pub fn randbytes_insecure(&mut self, len: usize) -> Vec<u8> {
            let mut out = Vec::with_capacity(len);
            while out.len() < len {
                let w = self.next_u64().to_le_bytes();
                let falta = len - out.len();
                out.extend_from_slice(&w[..falta.min(8)]);
            }
            out
        }

        /// Retorna `true` com probabilidade aproximada `p` (`p` limitado a `[0.0, 1.0]`).
        #[inline]
        pub fn bool_with_prob(&mut self, p: f64) -> bool {
            if p <= 0.0 || p.is_nan() {
                false
            } else if p >= 1.0 {
                true
            } else {
                self.random() < p
            }
        }
    }
}

// ============================================================================
// 12) Cyber Security & Binary Analysis — pub mod sec_utils
// ============================================================================
pub mod sec_utils {
    //! Utilitários de Análise Binária, Engenharia Reversa, Forense e Exploit Dev:
    //!
    //! - **Packing/Unpacking:** unsigned (`p8`..`p64`, `u8_le`..`u64_be`), signed (`p8_i8`..`p64_i64`,
    //!   `i8_le`..`i64_be`) e ponto flutuante (`p32_f32_*`, `p64_f64_*`, `f32_*`, `f64_*`).
    //! - **Cursor binário zero-copy:** [`BinReader`] e [`BinWriter`] com [`Endian`] e alinhamento.
    //! - **Codificação:** Hex, Base64 estrito (RFC 4648) e Base64URL, [`hexdump`], [`hexdump_with`].
    //! - **Checksums não criptográficos:** [`crc32`], [`adler32`], [`fnv1a_64`].
    //! - **Padrões & Entropia:** De Bruijn verdadeiro ($B(26, 4)$ via algoritmo FKM em [`cyclic`],
    //!   [`cyclic_find`], [`de_bruijn`]), busca AOB com wildcards e extração de strings ASCII.
    //! - **Segurança de memória:** [`constant_time_eq`] e [`secure_zero`] (com limites do LLVM
    //!   documentados com transparência).

    use std::fmt::{self, Write as _};

    // ---------------- Packing / Unpacking Unsigned, Signed e Float ----------

    #[inline]
    pub const fn p8(v: u8) -> [u8; 1] { [v] }
    #[inline]
    pub const fn p16(v: u16) -> [u8; 2] { v.to_le_bytes() }
    #[inline]
    pub const fn p32(v: u32) -> [u8; 4] { v.to_le_bytes() }
    #[inline]
    pub const fn p64(v: u64) -> [u8; 8] { v.to_le_bytes() }
    #[inline]
    pub const fn p16_be(v: u16) -> [u8; 2] { v.to_be_bytes() }
    #[inline]
    pub const fn p32_be(v: u32) -> [u8; 4] { v.to_be_bytes() }
    #[inline]
    pub const fn p64_be(v: u64) -> [u8; 8] { v.to_be_bytes() }

    #[inline]
    pub fn u8_le(b: &[u8]) -> Option<u8> { b.first().copied() }
    #[inline]
    pub fn u16_le(b: &[u8]) -> Option<u16> {
        b.get(..2).map(|s| u16::from_le_bytes([s[0], s[1]]))
    }
    #[inline]
    pub fn u32_le(b: &[u8]) -> Option<u32> {
        b.get(..4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    #[inline]
    pub fn u64_le(b: &[u8]) -> Option<u64> {
        b.get(..8).map(|s| {
            u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }
    #[inline]
    pub fn u16_be(b: &[u8]) -> Option<u16> {
        b.get(..2).map(|s| u16::from_be_bytes([s[0], s[1]]))
    }
    #[inline]
    pub fn u32_be(b: &[u8]) -> Option<u32> {
        b.get(..4).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    #[inline]
    pub fn u64_be(b: &[u8]) -> Option<u64> {
        b.get(..8).map(|s| {
            u64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }

    // Signed packing & unpacking
    #[inline]
    pub const fn p8_i8(v: i8) -> [u8; 1] { v.to_le_bytes() }
    #[inline]
    pub const fn p16_i16(v: i16) -> [u8; 2] { v.to_le_bytes() }
    #[inline]
    pub const fn p32_i32(v: i32) -> [u8; 4] { v.to_le_bytes() }
    #[inline]
    pub const fn p64_i64(v: i64) -> [u8; 8] { v.to_le_bytes() }
    #[inline]
    pub const fn p16_i16_be(v: i16) -> [u8; 2] { v.to_be_bytes() }
    #[inline]
    pub const fn p32_i32_be(v: i32) -> [u8; 4] { v.to_be_bytes() }
    #[inline]
    pub const fn p64_i64_be(v: i64) -> [u8; 8] { v.to_be_bytes() }

    #[inline]
    pub fn i8_le(b: &[u8]) -> Option<i8> { b.first().map(|&x| x as i8) }
    #[inline]
    pub fn i16_le(b: &[u8]) -> Option<i16> {
        b.get(..2).map(|s| i16::from_le_bytes([s[0], s[1]]))
    }
    #[inline]
    pub fn i32_le(b: &[u8]) -> Option<i32> {
        b.get(..4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    #[inline]
    pub fn i64_le(b: &[u8]) -> Option<i64> {
        b.get(..8).map(|s| {
            i64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }
    #[inline]
    pub fn i16_be(b: &[u8]) -> Option<i16> {
        b.get(..2).map(|s| i16::from_be_bytes([s[0], s[1]]))
    }
    #[inline]
    pub fn i32_be(b: &[u8]) -> Option<i32> {
        b.get(..4).map(|s| i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    #[inline]
    pub fn i64_be(b: &[u8]) -> Option<i64> {
        b.get(..8).map(|s| {
            i64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }

    // Float packing & unpacking
    #[inline]
    pub fn p32_f32_le(v: f32) -> [u8; 4] { v.to_le_bytes() }
    #[inline]
    pub fn p32_f32_be(v: f32) -> [u8; 4] { v.to_be_bytes() }
    #[inline]
    pub fn p64_f64_le(v: f64) -> [u8; 8] { v.to_le_bytes() }
    #[inline]
    pub fn p64_f64_be(v: f64) -> [u8; 8] { v.to_be_bytes() }

    #[inline]
    pub fn f32_le(b: &[u8]) -> Option<f32> {
        b.get(..4).map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    #[inline]
    pub fn f32_be(b: &[u8]) -> Option<f32> {
        b.get(..4).map(|s| f32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    #[inline]
    pub fn f64_le(b: &[u8]) -> Option<f64> {
        b.get(..8).map(|s| {
            f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }
    #[inline]
    pub fn f64_be(b: &[u8]) -> Option<f64> {
        b.get(..8).map(|s| {
            f64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
        })
    }

    // ---------------- BinReader (zero-copy) & BinWriter --------------------

    /// Ordem de bytes utilizada por [`BinReader`] e [`BinWriter`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Endian {
        Little,
        Big,
    }

    /// Erros de parsing binário com indicação precisa de posição, necessidade e limites.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BinError {
        UnexpectedEof { needed: usize, remaining: usize },
        InvalidOffset { offset: usize, len: usize },
        InvalidAlignment { align: usize, pos: usize },
        InvalidUtf8,
        MissingNullTerminator,
    }

    impl fmt::Display for BinError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                BinError::UnexpectedEof { needed, remaining } => write!(
                    f,
                    "EOFError: necessários {} bytes, restam apenas {}",
                    needed, remaining
                ),
                BinError::InvalidOffset { offset, len } => {
                    write!(f, "IndexError: offset {} fora do buffer (len={})", offset, len)
                }
                BinError::InvalidAlignment { align, pos } => write!(
                    f,
                    "ValueError: alinhamento inválido {} na posição {}",
                    align, pos
                ),
                BinError::InvalidUtf8 => write!(f, "ValueError: sequência de bytes UTF-8 inválida"),
                BinError::MissingNullTerminator => {
                    write!(f, "ValueError: C-string sem terminador nulo")
                }
            }
        }
    }

    impl std::error::Error for BinError {}

    /// Parser binário zero-copy com cursor, controle de endianness, alinhamento e bounds checking.
    #[derive(Debug, Clone)]
    pub struct BinReader<'a> {
        data: &'a [u8],
        pos: usize,
        endian: Endian,
    }

    impl<'a> BinReader<'a> {
        #[inline]
        pub fn new(data: &'a [u8], endian: Endian) -> Self {
            BinReader {
                data,
                pos: 0,
                endian,
            }
        }

        #[inline]
        pub const fn pos(&self) -> usize { self.pos }
        #[inline]
        pub const fn len(&self) -> usize { self.data.len() }
        #[inline]
        pub const fn remaining(&self) -> usize { self.data.len() - self.pos }
        #[inline]
        pub const fn is_empty(&self) -> bool { self.remaining() == 0 }
        #[inline]
        pub const fn endian(&self) -> Endian { self.endian }
        #[inline]
        pub fn set_endian(&mut self, endian: Endian) { self.endian = endian; }
        #[inline]
        pub fn as_slice(&self) -> &'a [u8] { self.data }
        #[inline]
        pub fn remaining_slice(&self) -> &'a [u8] { &self.data[self.pos..] }

        pub fn seek(&mut self, offset: usize) -> Result<(), BinError> {
            if offset > self.data.len() {
                return Err(BinError::InvalidOffset {
                    offset,
                    len: self.data.len(),
                });
            }
            self.pos = offset;
            Ok(())
        }

        pub fn skip(&mut self, n: usize) -> Result<(), BinError> {
            let novo = self.pos.checked_add(n).ok_or(BinError::UnexpectedEof {
                needed: n,
                remaining: self.remaining(),
            })?;
            if novo > self.data.len() {
                return Err(BinError::UnexpectedEof {
                    needed: n,
                    remaining: self.remaining(),
                });
            }
            self.pos = novo;
            Ok(())
        }

        /// Avança o cursor até o próximo múltiplo de `alignment` (potência de 2).
        /// Retorna a quantidade de bytes pulados.
        pub fn align(&mut self, alignment: usize) -> Result<usize, BinError> {
            if alignment == 0 || !alignment.is_power_of_two() {
                return Err(BinError::InvalidAlignment {
                    align: alignment,
                    pos: self.pos,
                });
            }
            let mask = alignment - 1;
            let aligned = self
                .pos
                .checked_add(mask)
                .map(|v| v & !mask)
                .ok_or(BinError::InvalidAlignment {
                    align: alignment,
                    pos: self.pos,
                })?;
            if aligned > self.data.len() {
                return Err(BinError::UnexpectedEof {
                    needed: aligned - self.pos,
                    remaining: self.remaining(),
                });
            }
            let pulados = aligned - self.pos;
            self.pos = aligned;
            Ok(pulados)
        }

        /// Lê `n` bytes retornando uma fatia emprestada zero-copy.
        pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], BinError> {
            let fatia = self.peek_bytes(n)?;
            self.pos += n;
            Ok(fatia)
        }

        /// Espia `n` bytes à frente sem avançar o cursor (zero-copy).
        pub fn peek_bytes(&self, n: usize) -> Result<&'a [u8], BinError> {
            let fim = self.pos.checked_add(n).ok_or(BinError::UnexpectedEof {
                needed: n,
                remaining: self.remaining(),
            })?;
            if fim > self.data.len() {
                return Err(BinError::UnexpectedEof {
                    needed: n,
                    remaining: self.remaining(),
                });
            }
            Ok(&self.data[self.pos..fim])
        }

        /// Lê `len` bytes em um `offset` absoluto arbitrário sem alterar o cursor (zero-copy).
        pub fn read_at(&self, offset: usize, len: usize) -> Result<&'a [u8], BinError> {
            let fim = offset.checked_add(len).ok_or(BinError::InvalidOffset {
                offset,
                len: self.data.len(),
            })?;
            if fim > self.data.len() {
                return Err(BinError::InvalidOffset {
                    offset: fim,
                    len: self.data.len(),
                });
            }
            Ok(&self.data[offset..fim])
        }

        pub fn read_u8(&mut self) -> Result<u8, BinError> {
            Ok(self.read_bytes(1)?[0])
        }
        pub fn read_i8(&mut self) -> Result<i8, BinError> {
            Ok(self.read_bytes(1)?[0] as i8)
        }
        pub fn read_u16(&mut self) -> Result<u16, BinError> {
            let s = self.read_bytes(2)?;
            Ok(match self.endian {
                Endian::Little => u16::from_le_bytes([s[0], s[1]]),
                Endian::Big => u16::from_be_bytes([s[0], s[1]]),
            })
        }
        pub fn read_i16(&mut self) -> Result<i16, BinError> {
            Ok(self.read_u16()? as i16)
        }
        pub fn read_u32(&mut self) -> Result<u32, BinError> {
            let s = self.read_bytes(4)?;
            Ok(match self.endian {
                Endian::Little => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                Endian::Big => u32::from_be_bytes([s[0], s[1], s[2], s[3]]),
            })
        }
        pub fn read_i32(&mut self) -> Result<i32, BinError> {
            Ok(self.read_u32()? as i32)
        }
        pub fn read_u64(&mut self) -> Result<u64, BinError> {
            let s = self.read_bytes(8)?;
            Ok(match self.endian {
                Endian::Little => {
                    u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
                }
                Endian::Big => {
                    u64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
                }
            })
        }
        pub fn read_i64(&mut self) -> Result<i64, BinError> {
            Ok(self.read_u64()? as i64)
        }
        pub fn read_f32(&mut self) -> Result<f32, BinError> {
            Ok(f32::from_bits(self.read_u32()?))
        }
        pub fn read_f64(&mut self) -> Result<f64, BinError> {
            Ok(f64::from_bits(self.read_u64()?))
        }

        /// Lê bytes terminados em `\0` (zero-copy, exclui o `\0` final do slice retornado e
        /// posiciona o cursor logo após o `\0`).
        pub fn read_cstr(&mut self) -> Result<&'a [u8], BinError> {
            let resto = &self.data[self.pos..];
            let nul = resto.iter().position(|&b| b == 0).ok_or(BinError::MissingNullTerminator)?;
            let fatia = &resto[..nul];
            self.pos += nul + 1;
            Ok(fatia)
        }

        /// Lê uma C-string terminada em `\0` validando UTF-8 (zero-copy).
        pub fn read_cstr_utf8(&mut self) -> Result<&'a str, BinError> {
            let bytes = self.read_cstr()?;
            std::str::from_utf8(bytes).map_err(|_| BinError::InvalidUtf8)
        }

        /// Lê exatamente `len` bytes e valida como `&str` UTF-8 (zero-copy).
        pub fn read_utf8(&mut self, len: usize) -> Result<&'a str, BinError> {
            let bytes = self.read_bytes(len)?;
            std::str::from_utf8(bytes).map_err(|_| BinError::InvalidUtf8)
        }
    }

    /// Construtor sequencial de buffers binários com controle de endianness e alinhamento.
    #[derive(Debug, Clone)]
    pub struct BinWriter {
        buf: Vec<u8>,
        endian: Endian,
    }

    impl BinWriter {
        #[inline]
        pub fn new(endian: Endian) -> Self {
            BinWriter {
                buf: Vec::new(),
                endian,
            }
        }
        #[inline]
        pub fn with_capacity(cap: usize, endian: Endian) -> Self {
            BinWriter {
                buf: Vec::with_capacity(cap),
                endian,
            }
        }
        #[inline]
        pub fn len(&self) -> usize { self.buf.len() }
        #[inline]
        pub fn is_empty(&self) -> bool { self.buf.is_empty() }
        #[inline]
        pub fn as_slice(&self) -> &[u8] { &self.buf }
        #[inline]
        pub fn into_bytes(self) -> Vec<u8> { self.buf }
        #[inline]
        pub fn set_endian(&mut self, endian: Endian) { self.endian = endian; }

        #[inline]
        pub fn write_bytes(&mut self, data: &[u8]) { self.buf.extend_from_slice(data); }
        #[inline]
        pub fn write_u8(&mut self, v: u8) { self.buf.push(v); }
        #[inline]
        pub fn write_i8(&mut self, v: i8) { self.buf.push(v as u8); }
        #[inline]
        pub fn write_u16(&mut self, v: u16) {
            match self.endian {
                Endian::Little => self.buf.extend_from_slice(&v.to_le_bytes()),
                Endian::Big => self.buf.extend_from_slice(&v.to_be_bytes()),
            }
        }
        #[inline]
        pub fn write_i16(&mut self, v: i16) { self.write_u16(v as u16); }
        #[inline]
        pub fn write_u32(&mut self, v: u32) {
            match self.endian {
                Endian::Little => self.buf.extend_from_slice(&v.to_le_bytes()),
                Endian::Big => self.buf.extend_from_slice(&v.to_be_bytes()),
            }
        }
        #[inline]
        pub fn write_i32(&mut self, v: i32) { self.write_u32(v as u32); }
        #[inline]
        pub fn write_u64(&mut self, v: u64) {
            match self.endian {
                Endian::Little => self.buf.extend_from_slice(&v.to_le_bytes()),
                Endian::Big => self.buf.extend_from_slice(&v.to_be_bytes()),
            }
        }
        #[inline]
        pub fn write_i64(&mut self, v: i64) { self.write_u64(v as u64); }
        #[inline]
        pub fn write_f32(&mut self, v: f32) { self.write_u32(v.to_bits()); }
        #[inline]
        pub fn write_f64(&mut self, v: f64) { self.write_u64(v.to_bits()); }

        /// Escreve `bytes` e anexa terminador `\0`.
        pub fn write_cstr(&mut self, bytes: &[u8]) {
            self.buf.extend_from_slice(bytes);
            self.buf.push(0);
        }

        /// Preenche com `fill` até o tamanho atual ser múltiplo de `alignment` (potência de 2).
        pub fn write_padding_align(&mut self, alignment: usize, fill: u8) -> Result<(), BinError> {
            if alignment == 0 || !alignment.is_power_of_two() {
                return Err(BinError::InvalidAlignment {
                    align: alignment,
                    pos: self.buf.len(),
                });
            }
            let rem = self.buf.len() & (alignment - 1);
            if rem != 0 {
                let pad = alignment - rem;
                self.buf.resize(self.buf.len() + pad, fill);
            }
            Ok(())
        }
    }

    // ---------------- Checksums Não Criptográficos --------------------------

    /// Calcula CRC-32 padrão IEEE 802.3 (polinômio refletido `0xEDB88320`, não criptográfico).
    pub fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    /// Calcula Adler-32 (RFC 1950, checksum não criptográfico usado em zlib).
    pub fn adler32(data: &[u8]) -> u32 {
        const MOD_ADLER: u32 = 65_521;
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &byte in data {
            a = (a + byte as u32) % MOD_ADLER;
            b = (b + a) % MOD_ADLER;
        }
        (b << 16) | a
    }

    /// Calcula FNV-1a de 64 bits (hash rápido não criptográfico para fingerprinting/indexação).
    pub fn fnv1a_64(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
        for &b in data {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash
    }

    // ---------------- Hex, Base64 Estrito, Base64URL e Hexdump ------------

    pub fn hex_encode(dados: &[u8]) -> String {
        const TABELA: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(dados.len() * 2);
        for &b in dados {
            out.push(TABELA[(b >> 4) as usize] as char);
            out.push(TABELA[(b & 0x0F) as usize] as char);
        }
        out
    }

    fn val_nibble(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }

    pub fn hex_decode(hex_str: &str) -> Result<Vec<u8>, String> {
        let mut limpo = Vec::with_capacity(hex_str.len());
        let bytes = hex_str.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_whitespace() || b == b':' || b == b'-' {
                i += 1;
                continue;
            }
            if (b == b'0' || b == b'\\')
                && i + 1 < bytes.len()
                && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X')
            {
                i += 2;
                continue;
            }
            limpo.push(b);
            i += 1;
        }

        if limpo.len() % 2 != 0 {
            return Err(format!(
                "ValueError: string hexadecimal com número ímpar de dígitos ({})",
                limpo.len()
            ));
        }

        let mut saida = Vec::with_capacity(limpo.len() / 2);
        for par in limpo.chunks_exact(2) {
            let hi = val_nibble(par[0]).ok_or_else(|| {
                format!(
                    "ValueError: caractere inválido '{}' em hex_decode",
                    par[0] as char
                )
            })?;
            let lo = val_nibble(par[1]).ok_or_else(|| {
                format!(
                    "ValueError: caractere inválido '{}' em hex_decode",
                    par[1] as char
                )
            })?;
            saida.push((hi << 4) | lo);
        }
        Ok(saida)
    }

    /// Configuração de formatação para [`hexdump_with`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HexdumpConfig {
        pub bytes_per_line: usize,
        pub base_address: u64,
        pub uppercase_hex: bool,
        pub show_ascii: bool,
    }

    impl Default for HexdumpConfig {
        fn default() -> Self {
            HexdumpConfig {
                bytes_per_line: 16,
                base_address: 0,
                uppercase_hex: false,
                show_ascii: true,
            }
        }
    }

    /// Gera dump hexadecimal configurável (endereço base, largura, caixa alta, coluna ASCII).
    pub fn hexdump_with(dados: &[u8], cfg: &HexdumpConfig) -> String {
        if dados.is_empty() {
            return String::new();
        }
        let bpl = cfg.bytes_per_line.max(1);
        let metade = bpl / 2;
        let linhas = (dados.len() + bpl - 1) / bpl;
        let mut out = String::with_capacity(linhas * (bpl * 4 + 24));

        for (idx_bloco, fatia) in dados.chunks(bpl).enumerate() {
            let addr = cfg.base_address.wrapping_add((idx_bloco * bpl) as u64);
            if cfg.uppercase_hex {
                let _ = write!(out, "{:08X}  ", addr);
            } else {
                let _ = write!(out, "{:08x}  ", addr);
            }

            for i in 0..bpl {
                if bpl >= 2 && i == metade {
                    out.push(' ');
                }
                if i < fatia.len() {
                    if cfg.uppercase_hex {
                        let _ = write!(out, "{:02X} ", fatia[i]);
                    } else {
                        let _ = write!(out, "{:02x} ", fatia[i]);
                    }
                } else {
                    out.push_str("   ");
                }
            }

            if cfg.show_ascii {
                out.push_str(" |");
                for &b in fatia {
                    if (0x20..=0x7E).contains(&b) {
                        out.push(b as char);
                    } else {
                        out.push('.');
                    }
                }
                out.push('|');
            }

            if (idx_bloco + 1) < linhas {
                out.push('\n');
            }
        }
        out
    }

    /// Gera um dump hexadecimal canônico estilo `xxd -C` / `hexdump -C`.
    pub fn hexdump(dados: &[u8]) -> String {
        hexdump_with(dados, &HexdumpConfig::default())
    }

    pub fn base64_encode(dados: &[u8]) -> String {
        const ALFABETO: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(((dados.len() + 2) / 3) * 4);
        for bloco in dados.chunks(3) {
            let b0 = bloco[0] as u32;
            let b1 = if bloco.len() > 1 { bloco[1] as u32 } else { 0 };
            let b2 = if bloco.len() > 2 { bloco[2] as u32 } else { 0 };
            let triplo = (b0 << 16) | (b1 << 8) | b2;

            out.push(ALFABETO[((triplo >> 18) & 0x3F) as usize] as char);
            out.push(ALFABETO[((triplo >> 12) & 0x3F) as usize] as char);
            if bloco.len() > 1 {
                out.push(ALFABETO[((triplo >> 6) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
            if bloco.len() > 2 {
                out.push(ALFABETO[(triplo & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }

    fn val_b64_std(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    /// Decodifica Base64 padrão (RFC 4648) com validação estrita de padding (`=`), posição de
    /// padding apenas no último bloco e rejeição de bits residuais não nulos (ignora whitespace).
    /// Para rejeitar também espaços em branco, use [`base64_decode_strict`].
    pub fn base64_decode(b64: &str) -> Result<Vec<u8>, String> {
        let limpo: Vec<u8> = b64
            .bytes()
            .filter(|b| !b.is_ascii_whitespace())
            .collect();
        if limpo.is_empty() {
            return Ok(Vec::new());
        }
        if limpo.len() % 4 != 0 {
            return Err("ValueError: comprimento de Base64 inválido".to_string());
        }

        let total_chunks = limpo.len() / 4;
        let mut out = Vec::with_capacity(total_chunks * 3);

        for (idx, quad) in limpo.chunks_exact(4).enumerate() {
            let eh_ultimo = idx + 1 == total_chunks;
            let v0 = val_b64_std(quad[0]).ok_or("ValueError: caractere Base64 inválido")? as u32;
            let v1 = val_b64_std(quad[1]).ok_or("ValueError: caractere Base64 inválido")? as u32;

            if quad[2] == b'=' {
                if !eh_ultimo || quad[3] != b'=' {
                    return Err("ValueError: padding Base64 inválido".to_string());
                }
                if (v1 & 0x0F) != 0 {
                    return Err("ValueError: bits não-canônicos no padding Base64".to_string());
                }
                let triplo = (v0 << 18) | (v1 << 12);
                out.push(((triplo >> 16) & 0xFF) as u8);
            } else if quad[3] == b'=' {
                if !eh_ultimo {
                    return Err("ValueError: padding Base64 fora do bloco final".to_string());
                }
                let v2 =
                    val_b64_std(quad[2]).ok_or("ValueError: caractere Base64 inválido")? as u32;
                if (v2 & 0x03) != 0 {
                    return Err("ValueError: bits não-canônicos no padding Base64".to_string());
                }
                let triplo = (v0 << 18) | (v1 << 12) | (v2 << 6);
                out.push(((triplo >> 16) & 0xFF) as u8);
                out.push(((triplo >> 8) & 0xFF) as u8);
            } else {
                let v2 =
                    val_b64_std(quad[2]).ok_or("ValueError: caractere Base64 inválido")? as u32;
                let v3 =
                    val_b64_std(quad[3]).ok_or("ValueError: caractere Base64 inválido")? as u32;
                let triplo = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
                out.push(((triplo >> 16) & 0xFF) as u8);
                out.push(((triplo >> 8) & 0xFF) as u8);
                out.push((triplo & 0xFF) as u8);
            }
        }
        Ok(out)
    }

    /// Decodifica Base64 RFC 4648 estrito rejeitando também espaços em branco ou quebras de linha.
    pub fn base64_decode_strict(b64: &str) -> Result<Vec<u8>, String> {
        if b64.bytes().any(|b| b.is_ascii_whitespace()) {
            return Err("ValueError: whitespace não permitido em base64_decode_strict".to_string());
        }
        base64_decode(b64)
    }

    /// Codifica em Base64URL (RFC 4648 §5, `-` e `_`), com padding `=` opcional.
    pub fn base64url_encode(dados: &[u8], pad: bool) -> String {
        const ALFABETO: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::with_capacity(((dados.len() + 2) / 3) * 4);
        for bloco in dados.chunks(3) {
            let b0 = bloco[0] as u32;
            let b1 = if bloco.len() > 1 { bloco[1] as u32 } else { 0 };
            let b2 = if bloco.len() > 2 { bloco[2] as u32 } else { 0 };
            let triplo = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALFABETO[((triplo >> 18) & 0x3F) as usize] as char);
            out.push(ALFABETO[((triplo >> 12) & 0x3F) as usize] as char);
            if bloco.len() > 1 {
                out.push(ALFABETO[((triplo >> 6) & 0x3F) as usize] as char);
            } else if pad {
                out.push('=');
            }
            if bloco.len() > 2 {
                out.push(ALFABETO[(triplo & 0x3F) as usize] as char);
            } else if pad {
                out.push('=');
            }
        }
        out
    }

    /// Decodifica Base64URL (aceita com ou sem padding `=`, validando estritamente).
    pub fn base64url_decode(b64: &str) -> Result<Vec<u8>, String> {
        if b64.bytes().any(|b| b.is_ascii_whitespace() || b == b'+' || b == b'/') {
            return Err("ValueError: caractere inválido em base64url_decode".to_string());
        }
        if b64.is_empty() {
            return Ok(Vec::new());
        }
        let mut s: String = b64
            .chars()
            .map(|c| match c {
                '-' => '+',
                '_' => '/',
                _ => c,
            })
            .collect();
        if !s.contains('=') {
            match s.len() % 4 {
                0 => {}
                2 => s.push_str("=="),
                3 => s.push('='),
                _ => return Err("ValueError: comprimento de Base64URL inválido".to_string()),
            }
        }
        base64_decode_strict(&s)
    }

    // ---------------- XOR, Entropia & Busca de Padrões ---------------------

    pub fn xor_cyclic(dados: &[u8], chave: &[u8]) -> Vec<u8> {
        if chave.is_empty() {
            return dados.to_vec();
        }
        dados
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ chave[i % chave.len()])
            .collect()
    }

    #[inline]
    pub fn xor_single(dados: &[u8], chave: u8) -> Vec<u8> {
        dados.iter().map(|&b| b ^ chave).collect()
    }

    /// Aplica XOR cíclico diretamente no buffer mutável sem alocar memória.
    pub fn xor_in_place(dados: &mut [u8], chave: &[u8]) {
        if chave.is_empty() {
            return;
        }
        for (i, b) in dados.iter_mut().enumerate() {
            *b ^= chave[i % chave.len()];
        }
    }

    pub fn shannon_entropy(dados: &[u8]) -> f64 {
        if dados.is_empty() {
            return 0.0;
        }
        let mut freq = [0usize; 256];
        for &b in dados {
            freq[b as usize] += 1;
        }
        let total = dados.len() as f64;
        let mut entropia = 0.0;
        for &contagem in &freq {
            if contagem > 0 {
                let p = (contagem as f64) / total;
                entropia -= p * p.log2();
            }
        }
        entropia
    }

    #[inline]
    pub fn is_high_entropy(dados: &[u8], limiar: f64) -> bool {
        shannon_entropy(dados) >= limiar
    }

    /// Busca a primeira ocorrência exata de `needle` em `haystack`.
    #[inline]
    pub fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Retorna todos os offsets onde `needle` ocorre em `haystack` (permite sobreposição).
    pub fn find_all_patterns(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
        if needle.is_empty() || needle.len() > haystack.len() {
            return Vec::new();
        }
        let mut res = Vec::new();
        for (idx, w) in haystack.windows(needle.len()).enumerate() {
            if w == needle {
                res.push(idx);
            }
        }
        res
    }

    /// Interpreta uma assinatura hexadecimal AOB (ex.: `"48 8B ?? 89 ?"`), retornando `None`
    /// para bytes curinga (`?` ou `??`).
    pub fn parse_aob_pattern(pattern: &str) -> Result<Vec<Option<u8>>, String> {
        let mut out = Vec::new();
        for tok in pattern.split_whitespace() {
            if tok == "?" || tok == "??" {
                out.push(None);
            } else {
                let raw = tok
                    .strip_prefix("0x")
                    .or_else(|| tok.strip_prefix("0X"))
                    .unwrap_or(tok);
                if raw.len() != 2 {
                    return Err(format!("ValueError: token AOB inválido '{}'", tok));
                }
                let byte = u8::from_str_radix(raw, 16)
                    .map_err(|_| format!("ValueError: byte AOB inválido '{}'", tok))?;
                out.push(Some(byte));
            }
        }
        if out.is_empty() {
            return Err("ValueError: padrão AOB vazio".to_string());
        }
        Ok(out)
    }

    /// Procura a primeira ocorrência de um padrão AOB com wildcards (`Option<u8>`).
    pub fn find_wildcard_pattern(haystack: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
        if pattern.is_empty() {
            return Some(0);
        }
        haystack.windows(pattern.len()).position(|w| {
            w.iter()
                .zip(pattern.iter())
                .all(|(&b, pat)| pat.is_none_or(|p| p == b))
        })
    }

    /// Extrai todas as sequências ASCII imprimíveis (`0x20..=0x7E` ou `\t`) de comprimento
    /// `>= min_len` como fatias emprestadas `&str` (zero-copy).
    pub fn extract_ascii_strings(dados: &[u8], min_len: usize) -> Vec<&str> {
        let min_l = min_len.max(1);
        let mut res = Vec::new();
        let mut ini: Option<usize> = None;

        for (i, &b) in dados.iter().enumerate() {
            let imprimivel = (0x20..=0x7E).contains(&b) || b == b'\t';
            match (ini, imprimivel) {
                (None, true) => ini = Some(i),
                (Some(s), false) => {
                    if i - s >= min_l {
                        if let Ok(txt) = std::str::from_utf8(&dados[s..i]) {
                            res.push(txt);
                        }
                    }
                    ini = None;
                }
                _ => {}
            }
        }
        if let Some(s) = ini {
            if dados.len() - s >= min_l {
                if let Ok(txt) = std::str::from_utf8(&dados[s..]) {
                    res.push(txt);
                }
            }
        }
        res
    }

    // ---------------- Sequência De Bruijn Verdadeira (FKM Iterativo) -------

    /// Iterador interno baseado no algoritmo iterativo de Duval/FKM que emite uma
    /// sequência De Bruijn $B(k, n)$ em espaço $O(n)$ sem repetições até $k^n$ bytes.
    struct DeBruijnIter<'a> {
        alphabet: &'a [u8],
        n: usize,
        w: Vec<usize>,
        buf: Vec<u8>,
        buf_idx: usize,
        done: bool,
    }

    impl<'a> DeBruijnIter<'a> {
        fn new(alphabet: &'a [u8], n: usize) -> Self {
            if alphabet.is_empty() || n == 0 {
                return DeBruijnIter {
                    alphabet,
                    n,
                    w: Vec::new(),
                    buf: Vec::new(),
                    buf_idx: 0,
                    done: true,
                };
            }
            DeBruijnIter {
                alphabet,
                n,
                w: vec![0],
                buf: Vec::with_capacity(n),
                buf_idx: 0,
                done: false,
            }
        }

        fn avancar_lyndon(&mut self) -> bool {
            let k = self.alphabet.len();
            loop {
                if self.w.is_empty() {
                    return false;
                }
                let emit = self.n % self.w.len() == 0;
                if emit {
                    self.buf.clear();
                    for &idx in &self.w {
                        self.buf.push(self.alphabet[idx]);
                    }
                    self.buf_idx = 0;
                }

                // Expande ciclicamente `w` até comprimento `n`
                let orig_len = self.w.len();
                let mut i = 0;
                while self.w.len() < self.n {
                    let v = self.w[i % orig_len];
                    self.w.push(v);
                    i += 1;
                }

                // Remove sufixo de símbolos máximos `k - 1` e incrementa o último restante
                while self.w.last() == Some(&(k - 1)) {
                    self.w.pop();
                }
                if let Some(last) = self.w.last_mut() {
                    *last += 1;
                }

                if emit {
                    return true;
                }
            }
        }
    }

    impl<'a> Iterator for DeBruijnIter<'a> {
        type Item = u8;

        #[inline]
        fn next(&mut self) -> Option<u8> {
            if self.done {
                return None;
            }
            if self.buf_idx >= self.buf.len() && !self.avancar_lyndon() {
                self.done = true;
                return None;
            }
            let b = self.buf[self.buf_idx];
            self.buf_idx += 1;
            Some(b)
        }
    }

    /// Gera uma sequência De Bruijn $B(|alphabet|, n)$ de tamanho `tamanho` bytes (até o período
    /// completo $|alphabet|^n$; se `tamanho > |alphabet|^n`, repete ciclicamente).
    pub fn de_bruijn(alphabet: &[u8], n: usize, tamanho: usize) -> Vec<u8> {
        if alphabet.is_empty() || n == 0 || tamanho == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(tamanho);
        while out.len() < tamanho {
            let mut it = DeBruijnIter::new(alphabet, n);
            let mut produziu = false;
            for b in &mut it {
                out.push(b);
                produziu = true;
                if out.len() >= tamanho {
                    break;
                }
            }
            if !produziu {
                break;
            }
        }
        out
    }

    /// Gera um padrão De Bruijn verdadeiro $B(26, 4)$ sobre `b'a'..=b'z'` (456.976 posições
    /// únicas para janelas de 4 bytes, estilo `pwntools.cyclic`).
    pub fn cyclic(tamanho: usize) -> Vec<u8> {
        de_bruijn(b"abcdefghijklmnopqrstuvwxyz", 4, tamanho)
    }

    /// Localiza o offset exato de `subpadrao` dentro da sequência De Bruijn $B(26, 4)$
    /// completa ($456.976$ bytes) via streaming em memória $O(|subpadrao|)$ (sem limite artificial
    /// de 20.000 bytes).
    pub fn cyclic_find(subpadrao: &[u8]) -> Option<usize> {
        if subpadrao.is_empty() {
            return None;
        }
        let target_len = subpadrao.len();
        let alphabet = b"abcdefghijklmnopqrstuvwxyz";
        // Garante que qualquer subpadrão não existente no alfabeto retorne None rápido
        if !subpadrao.iter().all(|b| b.is_ascii_lowercase()) {
            return None;
        }
        let mut window: Vec<u8> = Vec::with_capacity(target_len);
        // Inclui até target_len - 1 bytes extras do início para suportar busca em fronteira
        let iter = DeBruijnIter::new(alphabet, 4).chain(
            DeBruijnIter::new(alphabet, 4).take(target_len.saturating_sub(1)),
        );
        for (pos, b) in iter.enumerate() {
            if window.len() < target_len {
                window.push(b);
            } else {
                window.rotate_left(1);
                window[target_len - 1] = b;
            }
            if window.len() == target_len && window.as_slice() == subpadrao {
                return Some(pos + 1 - target_len);
            }
        }
        None
    }

    /// Atalho para localizar um valor `u32` (little-endian, ex.: registrador EIP capturado em
    /// crash) dentro do padrão [`cyclic`].
    #[inline]
    pub fn cyclic_find_u32_le(val: u32) -> Option<usize> {
        cyclic_find(&val.to_le_bytes())
    }

    // ---------------- Segurança de Memória ---------------------------------

    /// Compara duas fatias de bytes evitando saída antecipada por conteúdo (retorna `false`
    /// imediatamente apenas se os comprimentos diferirem).
    ///
    /// **Observação sobre `std`/compilador:** utiliza `std::hint::black_box` no acumulador para
    /// desencorajar curto-circuito pelo otimizador, porém garantias estritas de tempo constante em
    /// nível de instrução dependem da arquitetura/compilador e exigiriam assembly dedicado.
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut diff: u8 = 0;
        for (&x, &y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        std::hint::black_box(diff) == 0
    }

    /// Sobrescreve `buf` com zeros utilizando `std::ptr::write_volatile` e barreira de compilador.
    ///
    /// **Limitações honestas:** impede a eliminação de *dead store* pelo LLVM sobre este slice,
    /// mas não limpa cópias prévias em registradores, stack ou linhas de cache da CPU.
    pub fn secure_zero(buf: &mut [u8]) {
        for b in buf.iter_mut() {
            // SAFETY: `b` é uma referência mutável única e válida derivada de `&mut [u8]`,
            // portanto está alinhada e pertence ao buffer vivo durante esta gravação.
            unsafe {
                std::ptr::write_volatile(b, 0u8);
            }
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

// ============================================================================
// 13) Dicionários e Conjuntos Ordenados (BTreeMap / BTreeSet)
// ============================================================================
pub mod sorted_utils {
    //! Interface "Python-like" para `BTreeMap` e `BTreeSet` (equivalente a
    //! `sortedcontainers.SortedDict/SortedSet` + `bisect`).
    //!
    //! **Nota de complexidade:** `BTreeMap` e `BTreeSet` na `std` não mantêm contadores de tamanho
    //! por subárvore (*order-statistic trees*). Por isso, `bisect_left` e `bisect_right` contam
    //! elementos no prefixo em tempo **$O(k)$** (pior caso $O(N)$), enquanto `nth_item` e `nth`
    //! escolhem a ponta mais próxima em tempo **$O(\min(i, N - i))$**.

    use std::borrow::Borrow;
    use std::collections::{BTreeMap, BTreeSet};
    use std::ops::Bound::{Excluded, Included, Unbounded};

    /// Iterador lazy seguro para `irange_iter` (vazio sem panic quando `min > max`).
    pub struct SortedRangeIter<I>(Option<I>);

    impl<I: Iterator> Iterator for SortedRangeIter<I> {
        type Item = I::Item;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.0.as_mut().and_then(Iterator::next)
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            match &self.0 {
                Some(it) => it.size_hint(),
                None => (0, Some(0)),
            }
        }
    }

    impl<I: DoubleEndedIterator> DoubleEndedIterator for SortedRangeIter<I> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.0.as_mut().and_then(DoubleEndedIterator::next_back)
        }
    }

    pub trait PySortedDict<K: Ord, V> {
        fn get_or<Q>(&self, key: &Q, default: V) -> V
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
            V: Clone;

        /// Fallback preguiçoso para `get`: executa `f()` apenas se a chave não existir.
        fn get_or_else<Q, F>(&self, key: &Q, f: F) -> V
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
            V: Clone,
            F: FnOnce() -> V;

        fn setdefault(&mut self, key: K, default: V) -> &mut V;

        fn pop<Q>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;

        fn update<I: IntoIterator<Item = (K, V)>>(&mut self, items: I);
        fn first_key(&self) -> Option<&K>;
        fn last_key(&self) -> Option<&K>;
        fn first_item(&self) -> Option<(&K, &V)>;
        fn last_item(&self) -> Option<(&K, &V)>;
        /// i-ésimo par em ordem crescente. Complexidade: $O(\min(i, N - i))$.
        fn nth_item(&self, i: usize) -> Option<(&K, &V)>;

        fn floor_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;
        fn ceiling_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;
        fn lower_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;
        fn higher_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;

        /// Quantidade de chaves `< key`. Complexidade: $O(k)$ onde $k$ é o índice retornado ($O(N)$ pior caso).
        fn bisect_left<Q>(&self, key: &Q) -> usize
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;
        /// Quantidade de chaves `<= key`. Complexidade: $O(k)$ onde $k$ é o índice retornado ($O(N)$ pior caso).
        fn bisect_right<Q>(&self, key: &Q) -> usize
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;

        fn irange<Q>(&self, min: &Q, max: &Q) -> Vec<(&K, &V)>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;

        /// Iterador lazy sobre o intervalo `[min, max]` sem alocar `Vec` (vazio se `min > max`).
        fn irange_iter<Q>(
            &self,
            min: &Q,
            max: &Q,
        ) -> SortedRangeIter<std::collections::btree_map::Range<'_, K, V>>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized;
    }

    impl<K: Ord, V> PySortedDict<K, V> for BTreeMap<K, V> {
        fn get_or<Q>(&self, key: &Q, default: V) -> V
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
            V: Clone,
        {
            self.get(key).cloned().unwrap_or(default)
        }

        fn get_or_else<Q, F>(&self, key: &Q, f: F) -> V
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
            V: Clone,
            F: FnOnce() -> V,
        {
            match self.get(key) {
                Some(v) => v.clone(),
                None => f(),
            }
        }

        fn setdefault(&mut self, key: K, default: V) -> &mut V {
            self.entry(key).or_insert(default)
        }

        fn pop<Q>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.remove(key)
        }

        fn update<I: IntoIterator<Item = (K, V)>>(&mut self, items: I) {
            for (k, v) in items {
                self.insert(k, v);
            }
        }

        fn first_key(&self) -> Option<&K> {
            self.keys().next()
        }
        fn last_key(&self) -> Option<&K> {
            self.keys().next_back()
        }
        fn first_item(&self) -> Option<(&K, &V)> {
            self.first_key_value()
        }
        fn last_item(&self) -> Option<(&K, &V)> {
            self.last_key_value()
        }
        fn nth_item(&self, i: usize) -> Option<(&K, &V)> {
            let n = self.len();
            if i >= n {
                None
            } else if i <= n / 2 {
                self.iter().nth(i)
            } else {
                self.iter().nth_back(n - 1 - i)
            }
        }
        fn floor_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Included(key)))
                .next_back()
                .map(|(k, _)| k)
        }
        fn ceiling_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Included(key), Unbounded))
                .next()
                .map(|(k, _)| k)
        }
        fn lower_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Excluded(key)))
                .next_back()
                .map(|(k, _)| k)
        }
        fn higher_key<Q>(&self, key: &Q) -> Option<&K>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Excluded(key), Unbounded))
                .next()
                .map(|(k, _)| k)
        }
        fn bisect_left<Q>(&self, key: &Q) -> usize
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Excluded(key))).count()
        }
        fn bisect_right<Q>(&self, key: &Q) -> usize
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Included(key))).count()
        }
        fn irange<Q>(&self, min: &Q, max: &Q) -> Vec<(&K, &V)>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.irange_iter(min, max).collect()
        }

        fn irange_iter<Q>(
            &self,
            min: &Q,
            max: &Q,
        ) -> SortedRangeIter<std::collections::btree_map::Range<'_, K, V>>
        where
            K: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            if min > max {
                SortedRangeIter(None)
            } else {
                SortedRangeIter(Some(self.range::<Q, _>((Included(min), Included(max)))))
            }
        }
    }

    pub trait PySortedSet<T: Ord> {
        fn add(&mut self, value: T);
        fn discard<Q>(&mut self, value: &Q)
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        fn update<I: IntoIterator<Item = T>>(&mut self, items: I);
        /// i-ésimo elemento em ordem crescente. Complexidade: $O(\min(i, N - i))$.
        fn nth(&self, i: usize) -> Option<&T>;
        fn floor<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        fn ceiling<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        fn lower<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        fn higher<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        /// Quantidade de elementos `< value`. Complexidade: $O(k)$ ($O(N)$ pior caso).
        fn bisect_left<Q>(&self, value: &Q) -> usize
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        /// Quantidade de elementos `<= value`. Complexidade: $O(k)$ ($O(N)$ pior caso).
        fn bisect_right<Q>(&self, value: &Q) -> usize
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        fn irange<Q>(&self, min: &Q, max: &Q) -> Vec<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
        /// Iterador lazy sobre `[min, max]` sem alocar `Vec` (vazio se `min > max`).
        fn irange_iter<Q>(
            &self,
            min: &Q,
            max: &Q,
        ) -> SortedRangeIter<std::collections::btree_set::Range<'_, T>>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized;
    }

    impl<T: Ord> PySortedSet<T> for BTreeSet<T> {
        fn add(&mut self, value: T) {
            self.insert(value);
        }
        fn discard<Q>(&mut self, value: &Q)
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.remove(value);
        }
        fn update<I: IntoIterator<Item = T>>(&mut self, items: I) {
            self.extend(items);
        }
        fn nth(&self, i: usize) -> Option<&T> {
            let n = self.len();
            if i >= n {
                None
            } else if i <= n / 2 {
                self.iter().nth(i)
            } else {
                self.iter().nth_back(n - 1 - i)
            }
        }
        fn floor<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Included(value))).next_back()
        }
        fn ceiling<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Included(value), Unbounded)).next()
        }
        fn lower<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Excluded(value))).next_back()
        }
        fn higher<Q>(&self, value: &Q) -> Option<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Excluded(value), Unbounded)).next()
        }
        fn bisect_left<Q>(&self, value: &Q) -> usize
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Excluded(value))).count()
        }
        fn bisect_right<Q>(&self, value: &Q) -> usize
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.range::<Q, _>((Unbounded, Included(value))).count()
        }
        fn irange<Q>(&self, min: &Q, max: &Q) -> Vec<&T>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            self.irange_iter(min, max).collect()
        }

        fn irange_iter<Q>(
            &self,
            min: &Q,
            max: &Q,
        ) -> SortedRangeIter<std::collections::btree_set::Range<'_, T>>
        where
            T: Borrow<Q>,
            Q: Ord + ?Sized,
        {
            if min > max {
                SortedRangeIter(None)
            } else {
                SortedRangeIter(Some(self.range::<Q, _>((Included(min), Included(max)))))
            }
        }
    }
}

// ============================================================================
// 14) sql_utils — Composição estruturada parametrizada de SQL (sem driver)
// ============================================================================
pub mod sql_utils {
    //! Utilitários agnósticos de driver para composição parametrizada de consultas SQL em `std` pura.
    //!
    //! **Escopo realista (sem crates externas):**
    //! - Fornece identificação de dialeto ([`SqlDialect`]), organização estruturada de parâmetros
    //!   ([`SqlParam`], [`SqlQuery`]), construtor [`SqlBuilder`], templates com placeholders nomeados
    //!   ([`SqlTemplate`]) e validação sintática limitada de identificadores ([`SqlIdent`]).
    //! - **Não** contém engine SQL, **não** implementa drivers fictícios para SQLite/PostgreSQL/MySQL,
    //!   **não** executa consultas nem oferece função de escape de strings como pseudo-proteção
    //!   contra SQL Injection. Os valores de parâmetros são sempre mantidos separados da string SQL
    //!   para ligação (*bind*) segura por um driver externo futuro.

    use std::collections::HashMap;
    use std::fmt;

    /// Dialeto SQL alvo para geração de placeholders e citação de identificadores.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SqlDialect {
        /// `$1`, `$2`, ... e identificadores entre `"..."`
        Postgres,
        /// `?` e identificadores entre `"..."`
        Sqlite,
        /// `?` e identificadores entre `` `...` ``
        MySql,
        /// `@p1`, `@p2`, ... e identificadores entre `[...]`
        SqlServer,
    }

    impl SqlDialect {
        /// Gera o marcador posicional para o parâmetro `index_1_based` (1, 2, ...).
        pub fn placeholder(self, index_1_based: usize) -> String {
            match self {
                SqlDialect::Postgres => format!("${}", index_1_based),
                SqlDialect::Sqlite | SqlDialect::MySql => "?".to_string(),
                SqlDialect::SqlServer => format!("@p{}", index_1_based),
            }
        }
    }

    /// Erros de composição ou validação sintática de consulta SQL.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SqlError {
        InvalidIdentifier(String),
        MissingParameter(String),
        InvalidTemplate(String),
        EmptyList,
    }

    impl fmt::Display for SqlError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SqlError::InvalidIdentifier(id) => {
                    write!(f, "ValueError: identificador SQL inválido '{}'", id)
                }
                SqlError::MissingParameter(nome) => {
                    write!(f, "KeyError: parâmetro SQL ':'{} ausente", nome)
                }
                SqlError::InvalidTemplate(msg) => {
                    write!(f, "ValueError: template SQL inválido ({})", msg)
                }
                SqlError::EmptyList => {
                    write!(f, "ValueError: lista de valores para cláusula IN não pode ser vazia")
                }
            }
        }
    }

    impl std::error::Error for SqlError {}

    /// Valor de parâmetro SQL tipado mantido separado da string de consulta.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SqlParam {
        Null,
        Bool(bool),
        Int(i64),
        Float(f64),
        Text(String),
        Bytes(Vec<u8>),
    }

    impl From<bool> for SqlParam {
        fn from(v: bool) -> Self { SqlParam::Bool(v) }
    }
    impl From<i32> for SqlParam {
        fn from(v: i32) -> Self { SqlParam::Int(v as i64) }
    }
    impl From<i64> for SqlParam {
        fn from(v: i64) -> Self { SqlParam::Int(v) }
    }
    impl From<f64> for SqlParam {
        fn from(v: f64) -> Self { SqlParam::Float(v) }
    }
    impl From<&str> for SqlParam {
        fn from(v: &str) -> Self { SqlParam::Text(v.to_string()) }
    }
    impl From<String> for SqlParam {
        fn from(v: String) -> Self { SqlParam::Text(v) }
    }
    impl From<Vec<u8>> for SqlParam {
        fn from(v: Vec<u8>) -> Self { SqlParam::Bytes(v) }
    }
    impl<T: Into<SqlParam>> From<Option<T>> for SqlParam {
        fn from(v: Option<T>) -> Self {
            match v {
                Some(x) => x.into(),
                None => SqlParam::Null,
            }
        }
    }

    /// Identificador SQL validado sintaticamente (`[A-Za-z_][A-Za-z0-9_]*`, máximo 128 chars
    /// por segmento).
    ///
    /// **Nota:** trata-se exclusivamente de validação léxica básica para nomes de tabelas/colunas;
    /// ela **não** substitui controle de acesso nem listas brancas específicas da aplicação.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SqlIdent {
        partes: Vec<String>,
    }

    fn is_valid_ident_part(s: &str) -> bool {
        if s.is_empty() || s.len() > 128 {
            return false;
        }
        let mut bytes = s.bytes();
        let first = match bytes.next() {
            Some(b) => b,
            None => return false,
        };
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return false;
        }
        bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }

    impl SqlIdent {
        /// Valida um identificador simples (ex.: `"users"` ou `"user_id"`).
        pub fn new(name: &str) -> Result<Self, SqlError> {
            if !is_valid_ident_part(name) {
                return Err(SqlError::InvalidIdentifier(name.to_string()));
            }
            Ok(SqlIdent {
                partes: vec![name.to_string()],
            })
        }

        /// Valida um identificador opcionalmente qualificado por ponto (ex.: `"public.users"`).
        pub fn new_qualified(name: &str) -> Result<Self, SqlError> {
            let partes: Vec<String> = name.split('.').map(String::from).collect();
            if partes.is_empty() || partes.iter().any(|p| !is_valid_ident_part(p)) {
                return Err(SqlError::InvalidIdentifier(name.to_string()));
            }
            Ok(SqlIdent { partes })
        }

        /// Formata o identificador com as aspas adequadas para `dialect`.
        pub fn quote(&self, dialect: SqlDialect) -> String {
            let quoted: Vec<String> = self
                .partes
                .iter()
                .map(|p| match dialect {
                    SqlDialect::Postgres | SqlDialect::Sqlite => format!("\"{}\"", p),
                    SqlDialect::MySql => format!("`{}`", p),
                    SqlDialect::SqlServer => format!("[{}]", p),
                })
                .collect();
            quoted.join(".")
        }

        /// Retorna a forma textual sem aspas.
        pub fn as_raw(&self) -> String {
            self.partes.join(".")
        }
    }

    /// Consulta parametrizada final pronta para ser entregue a um driver SQL externo.
    #[derive(Debug, Clone, PartialEq)]
    pub struct SqlQuery {
        pub sql: String,
        pub params: Vec<SqlParam>,
    }

    /// Construtor estruturado que preserva a separação estrita entre fragmentos SQL e parâmetros.
    #[derive(Debug, Clone)]
    pub struct SqlBuilder {
        dialect: SqlDialect,
        sql: String,
        params: Vec<SqlParam>,
    }

    impl SqlBuilder {
        pub fn new(dialect: SqlDialect) -> Self {
            SqlBuilder {
                dialect,
                sql: String::new(),
                params: Vec::new(),
            }
        }

        /// Anexa um fragmento SQL estático ao buffer da consulta.
        pub fn push_sql(&mut self, fragment: &str) -> &mut Self {
            self.sql.push_str(fragment);
            self
        }

        /// Anexa um identificador validado (com citação própria do dialeto).
        pub fn push_ident(&mut self, ident: &SqlIdent) -> &mut Self {
            self.sql.push_str(&ident.quote(self.dialect));
            self
        }

        /// Registra `param` no vetor de bind e emite o placeholder correto do dialeto no SQL.
        pub fn push_param<P: Into<SqlParam>>(&mut self, param: P) -> &mut Self {
            self.params.push(param.into());
            let ph = self.dialect.placeholder(self.params.len());
            self.sql.push_str(&ph);
            self
        }

        /// Emite `(p1, p2, ...)` para uma cláusula `IN` parametrizada (rejeita lista vazia).
        pub fn push_in_list<I, P>(&mut self, items: I) -> Result<&mut Self, SqlError>
        where
            I: IntoIterator<Item = P>,
            P: Into<SqlParam>,
        {
            let mut it = items.into_iter().peekable();
            if it.peek().is_none() {
                return Err(SqlError::EmptyList);
            }
            self.sql.push('(');
            for (idx, item) in it.enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.push_param(item);
            }
            self.sql.push(')');
            Ok(self)
        }

        /// Anexa ` WHERE <ident> = <placeholder>`.
        pub fn where_eq<P: Into<SqlParam>>(&mut self, col: &SqlIdent, val: P) -> &mut Self {
            self.sql.push_str(" WHERE ");
            self.push_ident(col);
            self.sql.push_str(" = ");
            self.push_param(val);
            self
        }

        /// Anexa ` AND <ident> = <placeholder>`.
        pub fn and_eq<P: Into<SqlParam>>(&mut self, col: &SqlIdent, val: P) -> &mut Self {
            self.sql.push_str(" AND ");
            self.push_ident(col);
            self.sql.push_str(" = ");
            self.push_param(val);
            self
        }

        /// Finaliza devolvendo [`SqlQuery`] (`sql` + `params` separados).
        pub fn build(self) -> SqlQuery {
            SqlQuery {
                sql: self.sql,
                params: self.params,
            }
        }
    }

    #[derive(Debug, Clone)]
    enum TemplatePart {
        Literal(String),
        NamedParam(String),
    }

    /// Template SQL com placeholders nomeados (`:nome`), ignorando literais `'...'`,
    /// identificadores `"..."` e casts `::tipo`.
    #[derive(Debug, Clone)]
    pub struct SqlTemplate {
        parts: Vec<TemplatePart>,
    }

    impl SqlTemplate {
        /// Analisa uma string de template com marcadores `:nome`.
        pub fn new(template: &str) -> Result<Self, SqlError> {
            let mut parts = Vec::new();
            let mut lit = String::new();
            let bytes = template.as_bytes();
            let mut i = 0usize;
            let mut in_single = false;
            let mut in_double = false;

            while i < bytes.len() {
                let b = bytes[i];
                if in_single {
                    lit.push(b as char);
                    if b == b'\'' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                            lit.push('\'');
                            i += 2;
                            continue;
                        }
                        in_single = false;
                    }
                    i += 1;
                    continue;
                }
                if in_double {
                    lit.push(b as char);
                    if b == b'"' {
                        in_double = false;
                    }
                    i += 1;
                    continue;
                }
                if b == b'\'' {
                    in_single = true;
                    lit.push('\'');
                    i += 1;
                    continue;
                }
                if b == b'"' {
                    in_double = true;
                    lit.push('"');
                    i += 1;
                    continue;
                }
                if b == b':' {
                    // Ignora cast Postgres `::`
                    if i + 1 < bytes.len() && bytes[i + 1] == b':' {
                        lit.push_str("::");
                        i += 2;
                        continue;
                    }
                    if i + 1 < bytes.len()
                        && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_')
                    {
                        if !lit.is_empty() {
                            parts.push(TemplatePart::Literal(std::mem::take(&mut lit)));
                        }
                        let ini = i + 1;
                        let mut fim = ini;
                        while fim < bytes.len()
                            && (bytes[fim].is_ascii_alphanumeric() || bytes[fim] == b'_')
                        {
                            fim += 1;
                        }
                        let nome = std::str::from_utf8(&bytes[ini..fim])
                            .map_err(|_| SqlError::InvalidTemplate("UTF-8 inválido".to_string()))?;
                        parts.push(TemplatePart::NamedParam(nome.to_string()));
                        i = fim;
                        continue;
                    }
                }
                if let Some(ch) = template[i..].chars().next() {
                    lit.push(ch);
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }

            if in_single || in_double {
                return Err(SqlError::InvalidTemplate(
                    "aspas não fechadas no template".to_string(),
                ));
            }
            if !lit.is_empty() {
                parts.push(TemplatePart::Literal(lit));
            }
            Ok(SqlTemplate { parts })
        }

        /// Renderiza o template para o `dialect` substituindo cada `:nome` pelo placeholder
        /// posicional correspondente e populando `SqlQuery::params`.
        pub fn render(
            &self,
            dialect: SqlDialect,
            bindings: &[(&str, SqlParam)],
        ) -> Result<SqlQuery, SqlError> {
            let map: HashMap<&str, &SqlParam> = bindings.iter().map(|(k, v)| (*k, v)).collect();
            let mut sql = String::new();
            let mut params = Vec::new();
            for p in &self.parts {
                match p {
                    TemplatePart::Literal(s) => sql.push_str(s),
                    TemplatePart::NamedParam(name) => {
                        let val = map
                            .get(name.as_str())
                            .ok_or_else(|| SqlError::MissingParameter(name.clone()))?;
                        params.push((*val).clone());
                        sql.push_str(&dialect.placeholder(params.len()));
                    }
                }
            }
            Ok(SqlQuery { sql, params })
        }
    }
}

// ============================================================================
// Macros de Produtividade CyberSec, Coleções & Impressão
// ============================================================================

/// Converte uma string hexadecimal (aceitando espaços, `0x` ou `\x`) em `Vec<u8>`.
#[macro_export]
macro_rules! unhex {
    ($hex:expr) => {
        $crate::sec_utils::hex_decode($hex)
            .unwrap_or_else(|e| panic!("{}", e))
    };
}

/// Realiza um probe rápido em `(host, port)` com timeout padrão de `1.0s` (ou customizado).
#[macro_export]
macro_rules! tcp_probe {
    ($host:expr, $port:expr) => {
        $crate::net_utils::tcp_probe($host, $port, 1.0, true)
    };
    ($host:expr, $port:expr, $timeout:expr) => {
        $crate::net_utils::tcp_probe($host, $port, $timeout, true)
    };
}

/// Sobe um servidor TCP de teste em background numa thread dedicada para atender uma conexão.
#[macro_export]
macro_rules! tcp_server_spawn {
    ($addr:expr, |$cli:ident, $peer:ident| $body:block) => {{
        let srv = $crate::net_utils::TcpServer::listen($addr)
            .expect("IOError: falha ao iniciar TcpServer em tcp_server_spawn!");
        let local = srv
            .local_addr()
            .expect("IOError: falha ao obter local_addr em tcp_server_spawn!");
        let handle = ::std::thread::spawn(move || {
            if let Ok((mut $cli, $peer)) = srv.accept() {
                $body
            }
        });
        (local, handle)
    }};
    ($addr:expr, |$cli:ident| $body:block) => {{
        $crate::tcp_server_spawn!($addr, |$cli, _peer| $body)
    }};
}

/// Cria um `BTreeMap` (dicionário ordenado): `sorted_dict!{ "b" => 2, "a" => 1 }`.
#[macro_export]
macro_rules! sorted_dict {
    () => { ::std::collections::BTreeMap::new() };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut m = ::std::collections::BTreeMap::new();
        $( m.insert($k, $v); )+
        m
    }};
}

/// Cria um `BTreeSet` (conjunto ordenado): `sorted_set![3, 1, 2]`.
#[macro_export]
macro_rules! sorted_set {
    () => { ::std::collections::BTreeSet::new() };
    ($($x:expr),+ $(,)?) => {{
        let mut s = ::std::collections::BTreeSet::new();
        $( s.insert($x); )+
        s
    }};
}

/// Cria um `HashMap` (dicionário não ordenado) estilo Python: `dict!{ "a" => 1, "b" => 2 }`.
#[macro_export]
macro_rules! dict {
    () => { ::std::collections::HashMap::new() };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut m = ::std::collections::HashMap::new();
        $( m.insert($k, $v); )+
        m
    }};
}

/// Cria um `HashSet` (conjunto não ordenado) estilo Python: `set![1, 2, 3]`.
#[macro_export]
macro_rules! set {
    () => { ::std::collections::HashSet::new() };
    ($($x:expr),+ $(,)?) => {{
        let mut s = ::std::collections::HashSet::new();
        $( s.insert($x); )+
        s
    }};
}

/// Cria uma `VecDeque` estilo Python: `deque![1, 2, 3]`.
#[macro_export]
macro_rules! deque {
    () => { ::std::collections::VecDeque::new() };
    ($($x:expr),+ $(,)?) => {{
        let mut d = ::std::collections::VecDeque::new();
        $( d.push_back($x); )+
        d
    }};
}

/// Macro ergonômica de impressão estilo `print(a, b, c)` do Python (separa múltiplos
/// argumentos com espaço e quebra linha ao final, sem exigir string de formatação).
#[macro_export]
macro_rules! py_print {
    () => {
        ::std::println!()
    };
    ($first:expr $(, $rest:expr)* $(,)?) => {{
        let mut out = ::std::format!("{}", $first);
        $(
            out.push(' ');
            ::std::fmt::Write::write_fmt(&mut out, ::std::format_args!("{}", $rest))
                .expect("formatação Display");
        )*
        ::std::println!("{}", out);
    }};
}

// ============================================================================
// Prelude — import único e ergonômico: `use pyrsutils2::prelude::*;`
// ============================================================================
pub mod prelude {
    // v1.0..v1.5.1 preservados integralmente
    pub use super::arithmetic::{abs, div, divmod, mult, pow, root, sub, sum, ArithError, NumeroPy};
    pub use super::builtins::{
        all, any, enumerate, histogram, len, max, min, print, reversed, sorted, type_of, zip,
        PyLen,
    };
    pub use super::list_utils::PyList;
    pub use super::matrix_utils::Matrix;
    pub use super::range_utils::{range, range_from, range_step, PyRange};
    pub use super::string_utils::{
        count, endswith, find, is_palindrome, join, lower, replace, split, startswith, strip,
        upper,
    };
    pub use super::{input, try_input, InputError};
    // v1.1 — tempo e rede
    pub use super::net_utils::{getaddrinfo, gethostbyname, TcpClient, TcpServer, UdpSock};
    pub use super::time_utils::{
        gmtime, gmtime_now, mktime_utc, perf_counter, perf_counter_ns, sleep, strftime, time,
        timeit, timeit_repeat, StructTime,
    };
    // v1.2 / v1.4 — sistema de arquivos + I/O rápido de texto
    pub use super::fs_utils::{
        abspath, append_text, basename, chdir, copy, dirname, exists, expanduser, getcwd, glob,
        isdir, isfile, islink, join_path, listdir, makedirs, mkdir, read_text, readlines,
        readlink, realpath, remove, rename, rmdir, rmtree, splitext, walk, write_text, Walk,
        WalkItem,
    };
    #[cfg(unix)]
    pub use super::fs_utils::{chmod, stat, symlink, Stat};
    // v1.4 — números aleatórios
    pub use super::random_utils::{choice, randint, random, seed, shuffle};
    // v1.5 — Cyber Security, Exploit Dev, Packet Crafting, PCAP & Linux /proc
    pub use super::net_utils::{
        ipv4_tcp_checksum, parse_cidr, parse_ports, parse_proc_net_arp_str,
        parse_proc_net_tcp_str, proc_net_arp, proc_net_tcp, rfc1071_checksum, tcp_probe, tcp_scan,
        Ipv4Cidr, Ipv4TcpPacket, PcapWriter, PortScanResult, ProcArpEntry, ProcTcpEntry, TCP_ACK,
        TCP_CWR, TCP_ECE, TCP_FIN, TCP_PSH, TCP_RST, TCP_SYN, TCP_URG,
    };
    pub use super::sec_utils::{
        base64_decode, base64_encode, constant_time_eq, cyclic, cyclic_find, hex_decode,
        hex_encode, hexdump, is_high_entropy, p16, p16_be, p32, p32_be, p64, p64_be, p8,
        secure_zero, shannon_entropy, u16_be, u16_le, u32_be, u32_le, u64_be, u64_le, u8_le,
        xor_cyclic, xor_single,
    };
    // v1.5.1
    pub use super::sorted_utils::{PySortedDict, PySortedSet};

    // ---------------- v2.0.0: Novas adições curadas sem colisões ----------------
    pub use super::arithmetic::{
        ceil, ceil_div, checked_abs, checked_add, checked_div, checked_gcd, checked_lcm,
        checked_mult, checked_mul, checked_pow, checked_rem, checked_sub, checked_sum,
        f64_to_i64_checked, floor, floor_div, gcd, is_close, lcm, py_divmod, round,
        saturating_add, saturating_mul, saturating_pow, saturating_sub, trunc, try_cast,
        wrapping_add, wrapping_mul, wrapping_pow, wrapping_sub, PyFloat, PyInt,
    };
    pub use super::builtins::{
        clamp, contains_in, count_in, enumerate_from, eprint, find_in, max_by_key, max_ord,
        min_by_key, min_ord, product, sorted_by_key, sorted_desc, sorted_f64_total, sorted_ord,
        try_clamp, try_max, try_min, try_sorted, CompareError,
    };
    pub use super::collection_utils::{
        KeyError, MinHeap, PriorityQueue, PyDeque, PyDict, PySet,
    };
    pub use super::concurrency_utils::{
        bounded_channel, parallel_for_each, parallel_map, unbounded_channel, ConcurrencyError,
        RateLimiter, Shared, ThreadPool,
    };
    pub use super::env_utils::{
        arch, available_parallelism, current_dir, current_exe, has_var, home_dir, os_family,
        os_name, set_current_dir, split_paths_env, temp_dir, var, var_or, var_os, var_parse,
        var_strict,
    };
    pub use super::fs_utils::{
        append_bytes, copy_tree, file_size, metadata, read_bytes, read_bytes_limited,
        read_lines_lazy, read_text_limited, touch, write_bytes, write_bytes_atomic,
        write_text_atomic, FileMetadata,
    };
    pub use super::input_utils::{
        eprint_line, input_or, read_line, read_line_limited, read_stdin_bytes, read_stdin_lines,
        stderr_is_tty, stdin_is_tty, stdout_is_tty, try_input_limited, write_stderr_bytes,
        write_stdout_bytes,
    };
    pub use super::io_utils::{
        copy_stream_limited, for_each_line, read_to_string_limited, read_to_vec_limited,
        BufferedReader, BufferedWriter,
    };
    pub use super::iterator_utils::{
        batched, count_from, islice, zip_longest, EitherOrBoth, PyIterExt,
    };
    pub use super::matrix_utils::MatrixError;
    pub use super::net_utils::{
        ipv4_udp_checksum, parse_cidr_v6, tcp_probe_detailed, tcp_scan_with_config,
        try_ipv4_tcp_checksum, DetailedPortScanResult, IpCidr, Ipv4Range, Ipv4UdpPacket,
        Ipv6Cidr, NetPacketError, PcapPacket, PcapReader, PortState, ScanConfig,
        StreamingPcapWriter,
    };
    pub use super::process_utils::{
        run as run_command, run_checked as run_command_checked, run_in_dir, run_pipeline,
        run_with_stdin, ManagedChild, ProcessBuilder, ProcessError, ProcessOutput,
    };
    pub use super::random_utils::{
        os_random_bytes, randbytes_insecure, try_randint, uniform, PyRng,
    };
    pub use super::range_utils::{range_idx, range_idx_from, try_range_step, RangeError};
    pub use super::sec_utils::{
        adler32, base64_decode_strict, base64url_decode, base64url_encode, crc32,
        cyclic_find_u32_le, de_bruijn, extract_ascii_strings, f32_be, f32_le, f64_be, f64_le,
        find_all_patterns, find_pattern, find_wildcard_pattern, fnv1a_64, hexdump_with, i16_be,
        i16_le, i32_be, i32_le, i64_be, i64_le, i8_le, p16_i16, p16_i16_be, p32_f32_be,
        p32_f32_le, p32_i32, p32_i32_be, p64_f64_be, p64_f64_le, p64_i64, p64_i64_be, p8_i8,
        parse_aob_pattern, xor_in_place, BinError, BinReader, BinWriter, Endian, HexdumpConfig,
    };
    pub use super::sql_utils::{
        SqlBuilder, SqlDialect, SqlError, SqlIdent, SqlParam, SqlQuery, SqlTemplate,
    };
    pub use super::string_utils::{
        byte_len, center, char_count, contains as str_contains, is_alnum, is_alpha, is_ascii,
        is_ascii_digits, is_numeric, lstrip, lstrip_chars, pad_left, pad_right, partition,
        removeprefix, removesuffix, rfind, rpartition, rsplit, rstrip, rstrip_chars, split_lazy,
        split_whitespace, splitlines, splitlines_lazy, strip_chars, substring, substring_py,
    };
    pub use super::terminal_utils::{
        clear_screen, colorize, colorize_always, colors_enabled, confirm, print_error,
        print_info, print_ok, print_warn, select_menu, strip_ansi, style_text, Color, ColorMode,
        ProgressBar, Style,
    };
    pub use super::time_utils::{
        days_in_month, duration_from_secs, duration_to_secs, is_leap_year, perf_counter_ns_u128,
        try_mktime_utc, try_sleep, Deadline, TimeError, Timer,
    };
}

// ============================================================================
// Suíte Completa de Testes (Compatibilidade v1.0..v1.5.1 + Bateria v2.0.0)
// ============================================================================
#[cfg(test)]
mod tests {
    #[test]
    fn string_sempre_converte() {
        let texto = "ola mundo".to_string();
        let r: Result<String, _> = texto.parse();
        assert_eq!(r.unwrap(), "ola mundo");
    }

    #[test]
    fn inteiro_converte() {
        let r: Result<i32, _> = "42".parse();
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn float_converte() {
        let r: Result<f64, _> = "3.14".parse();
        assert_eq!(r.unwrap(), 3.14);
    }

    #[test]
    fn bool_converte() {
        let r: Result<bool, _> = "true".parse();
        assert_eq!(r.unwrap(), true);
    }

    #[test]
    fn inteiro_invalido_falha() {
        let r: Result<i32, _> = "abc".parse();
        assert!(r.is_err());
    }
}

#[cfg(test)]
mod tests_extensoes {
    use super::*;

    #[test]
    fn range_basico_e_contains_funcionam() {
        let r = range_utils::range(5);
        assert!(r.contains(3));
        assert!(!r.contains(5));
        let v: Vec<i64> = r.collect();
        assert_eq!(v, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn range_step_negativo_funciona() {
        let r = range_utils::range_step(10, 0, -2);
        assert!(r.contains(8));
        assert!(!r.contains(7));
        let v: Vec<i64> = r.collect();
        assert_eq!(v, vec![10, 8, 6, 4, 2]);
    }

    #[test]
    fn range_tem_len_sem_iterar() {
        use builtins::len;
        assert_eq!(len(&range_utils::range(10)), 10);
        assert_eq!(len(&range_utils::range_step(10, 0, -2)), 5);
        assert_eq!(len(&range_utils::range_from(5, 5)), 0);
        assert_eq!(len(&range_utils::range_from(5, 2)), 0);
    }

    #[test]
    fn lista_append_pop_remove_extend_index() {
        use list_utils::PyList;
        let mut v = vec![1, 2, 3];
        v.append_item(4);
        assert_eq!(v, vec![1, 2, 3, 4]);
        assert_eq!(v.pop_at(None), Some(4));
        assert_eq!(v.pop_at(Some(0)), Some(1));
        assert_eq!(v, vec![2, 3]);
        assert!(v.remove_value(&3));
        assert_eq!(v, vec![2]);
        v.extend_from(vec![5, 6, 2]);
        assert_eq!(v.index_of(&5), Some(1));
        assert_eq!(v.count_value(&2), 2);
    }

    #[test]
    fn matriz_get_set_transpose_e_dot() {
        use matrix_utils::Matrix;
        let mut m = Matrix::new(2, 3, 0);
        m[(0, 0)] = 1;
        m[(1, 2)] = 9;
        assert_eq!(m.get(0, 0), Some(&1));
        assert_eq!(m.get(1, 2), Some(&9));
        let t = m.transpose();
        assert_eq!(t[(0, 0)], 1);
        assert_eq!(t[(2, 1)], 9);

        let prod = m.dot(&t).unwrap();
        assert_eq!((prod.linhas, prod.colunas), (2, 2));
        assert_eq!(prod[(0, 0)], 1);
        assert_eq!(prod[(1, 1)], 81);
    }

    #[test]
    fn builtins_completos() {
        use builtins::*;
        assert_eq!(min(vec![3, 1, 2]), Some(1));
        assert_eq!(max(vec![3, 1, 2]), Some(3));
        assert_eq!(sorted(vec![3, 1, 2]), vec![1, 2, 3]);
        let r: Vec<i32> = reversed(vec![1, 2, 3]).collect();
        assert_eq!(r, vec![3, 2, 1]);
        assert_eq!(len("olá"), 3);
        assert_eq!(len(&String::from("café")), 4);
        assert_eq!(len(&vec![10, 20]), 2);

        let pares: Vec<(usize, &str)> = enumerate(vec!["a", "b"]).collect();
        assert_eq!(pares, vec![(0, "a"), (1, "b")]);

        let zipado: Vec<(i32, i32)> = zip(vec![1, 2], vec![10, 20]).collect();
        assert_eq!(zipado, vec![(1, 10), (2, 20)]);

        assert!(any(vec![false, true, false]));
        assert!(all(vec![true, true]));

        let hist = histogram("banana".chars());
        assert_eq!(hist.get(&'a'), Some(&3));
        assert_eq!(hist.get(&'n'), Some(&2));
    }

    #[test]
    fn aritmetica_basica_e_erros_de_borda() {
        use arithmetic::*;
        assert_eq!(sum(vec![1usize, 2, 3]), 6usize);
        assert_eq!(sub(vec![10isize, 1, 2]), Some(7isize));
        assert_eq!(mult(vec![2, 3, 4]), 24);
        assert_eq!(div(10, 2), Ok(5));
        assert_eq!(div(10, 0), Err(ArithError::DivisaoPorZero));
        assert_eq!(divmod(7, 3), Ok((2, 1)));
        assert_eq!(abs(-42i32), 42);
        assert_eq!(pow(2, 10), 1024);
        assert!((root(9.0, 2.0).unwrap() - 3.0).abs() < 1e-9);
        assert!((root(-8.0, 3.0).unwrap() + 2.0).abs() < 1e-9);
        assert_eq!(root(-8.0, 2.5), Err(ArithError::RaizDeNegativoComIndicePar));
        assert_eq!(root(9.0, 0.0), Err(ArithError::IndiceDeRaizInvalido));
    }

    #[test]
    fn strings_utilitarios_completos() {
        use string_utils::*;
        assert_eq!(split("a,b,c", ","), vec!["a", "b", "c"]);
        assert_eq!(strip("  oi  "), "oi");
        assert_eq!(join(", ", vec!["a", "b", "c"]), "a, b, c");
        assert_eq!(join(", ", Vec::<&str>::new()), "");
        assert_eq!(replace("banana", "na", "NA"), "baNANA");
        assert_eq!(lower("OlÁ"), "olá");
        assert_eq!(upper("olá"), "OLÁ");
        assert!(startswith("rust_lang", "rust"));
        assert!(endswith("rust_lang", "lang"));
        assert_eq!(find("café_bom", "bom"), 5);
        assert_eq!(find("abc", "z"), -1);
        assert_eq!(count("banana", "na"), 2);
        assert!(is_palindrome("reviver"));
        assert!(is_palindrome("osso"));
        assert!(!is_palindrome("python"));
    }

    class! {
        struct Contador {
            valor: i32
        }
        impl {
            fn incrementar(&mut self) {
                self.valor += 1;
            }
            fn valor_atual(&self) -> i32 {
                self.valor
            }
        }
    }

    class! {
        struct ContadorComPasso : Contador {
            passo: i32
        }
        impl {
            fn avancar(&mut self) {
                self.valor += self.passo;
            }
        }
    }

    #[test]
    fn class_macro_base_e_heranca_funcionam() {
        let mut c = Contador::new(0);
        c.incrementar();
        c.incrementar();
        assert_eq!(c.valor_atual(), 2);

        let mut cp = ContadorComPasso::new(Contador::new(10), 5);
        cp.incrementar();
        cp.avancar();
        assert_eq!(cp.valor_atual(), 16);
    }
}

#[cfg(test)]
mod tests_tempo_rede_random {
    use super::net_utils::*;
    use super::random_utils::*;
    use super::time_utils::*;

    #[test]
    fn time_e_perf_counter_avancam() {
        assert!(time() > 1.7e9);
        let t0 = perf_counter();
        sleep(0.02);
        assert!(perf_counter() - t0 >= 0.019);
        let n0 = perf_counter_ns();
        assert!(perf_counter_ns() >= n0);
    }

    #[test]
    fn macro_timeit_higienica_funciona() {
        let mut c = 0;
        let t = crate::timeit!(7, {
            c += 1;
        });
        assert_eq!(c, 7);
        assert!(t >= 0.0);
    }

    #[test]
    fn gmtime_e_strftime() {
        let t = gmtime(1_000_000_000);
        assert_eq!(strftime("%Y-%m-%d %H:%M:%S", &t), "2001-09-09 01:46:40");
        assert_eq!(mktime_utc(&t), 1_000_000_000);
    }

    #[test]
    fn tcp_e_udp_loopback() {
        let srv = TcpServer::listen("127.0.0.1:0").unwrap();
        let porta = srv.local_addr().unwrap().port();
        let th = std::thread::spawn(move || {
            let (mut cli, _) = srv.accept().unwrap();
            let d = cli.recv_exact(5).unwrap();
            cli.send(&d).unwrap();
        });
        let mut c = TcpClient::connect(("127.0.0.1", porta)).unwrap();
        c.set_timeout(Some(2.0)).unwrap();
        c.send("olá!").unwrap();
        assert_eq!(c.recv_exact(5).unwrap(), "olá!".as_bytes());
        th.join().unwrap();

        let a = UdpSock::bind("127.0.0.1:0").unwrap();
        let b = UdpSock::bind("127.0.0.1:0").unwrap();
        b.send_to("ping", a.local_addr().unwrap()).unwrap();
        let (msg, _) = a.recv_from(512).unwrap();
        assert_eq!(msg, b"ping");
    }

    #[test]
    fn random_seed_randint_choice_shuffle() {
        seed(42);
        for _ in 0..50 {
            let r = random();
            assert!((0.0..1.0).contains(&r));
            let n = randint(5, 10);
            assert!((5..=10).contains(&n));
        }
        let itens = [10, 20, 30, 40];
        assert!(choice(&itens).is_some());
        let mut v = vec![1, 2, 3, 4, 5, 6, 7, 8];
        shuffle(&mut v);
        assert_eq!(v.len(), 8);
    }
}

#[cfg(test)]
mod tests_fs {
    use super::fs_utils::*;
    use std::fs;

    fn area_de_teste(sufixo: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pyrsutils_v2_fs_{}_{}",
            std::process::id(),
            sufixo
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn io_texto_e_walk_seguro_contra_loop_symlink() {
        let dir = area_de_teste("io_walk");
        let arq = dir.join("notas.txt");
        write_text(&arq, "linha 1\n").unwrap();
        append_text(&arq, "linha 2\n").unwrap();
        assert_eq!(read_text(&arq).unwrap(), "linha 1\nlinha 2\n");
        assert_eq!(readlines(&arq).unwrap(), vec!["linha 1", "linha 2"]);

        #[cfg(unix)]
        {
            let link_ciclico = dir.join("loop_infinito");
            symlink(&dir, &link_ciclico).unwrap();
        }

        let mut passos = 0;
        for item in walk(&dir) {
            let _ = item.unwrap();
            passos += 1;
            assert!(passos < 10, "walk entrou em loop infinito com symlink!");
        }
        rmtree(&dir).unwrap();
    }
}

#[cfg(test)]
mod tests_v15_cybersec_e_rede {
    use super::net_utils::*;
    use super::sec_utils::*;
    use std::net::Ipv4Addr;

    #[test]
    fn pwntools_recvuntil_sendlineafter_sem_overread() {
        let (addr, handle) = crate::tcp_server_spawn!("127.0.0.1:0", |cli| {
            cli.send(b"Bem-vindo ao CTF\nSenha: ").unwrap();
            let tentativa = cli.recvline_string(128).unwrap();
            if tentativa == "s3cr3t" {
                cli.send(b"FLAG{pwn_ok}\nRESTO_DO_STREAM").unwrap();
            }
        });

        let mut p = TcpClient::connect(addr).unwrap();
        p.set_timeout(Some(2.0)).unwrap();

        let prompt = p.sendlineafter(b"Senha: ", b"s3cr3t", 256).unwrap();
        assert!(prompt.ends_with(b"Senha: "));

        let flag = p.recvline_string(128).unwrap();
        assert_eq!(flag, "FLAG{pwn_ok}");

        let resto = p.recv_exact(15).unwrap();
        assert_eq!(&resto, b"RESTO_DO_STREAM");
        handle.join().unwrap();
    }

    #[test]
    fn cidr_ipv4_e_parse_ports_nmap() {
        let cidr = parse_cidr("192.168.10.0/30").unwrap();
        assert_eq!(cidr.network(), Ipv4Addr::new(192, 168, 10, 0));
        assert_eq!(cidr.broadcast(), Ipv4Addr::new(192, 168, 10, 3));
        assert_eq!(cidr.netmask(), Ipv4Addr::new(255, 255, 255, 252));
        assert!(cidr.contains(Ipv4Addr::new(192, 168, 10, 2)));
        assert!(!cidr.contains(Ipv4Addr::new(192, 168, 10, 4)));

        let hosts: Vec<Ipv4Addr> = cidr.hosts().collect();
        assert_eq!(
            hosts,
            vec![
                Ipv4Addr::new(192, 168, 10, 1),
                Ipv4Addr::new(192, 168, 10, 2)
            ]
        );

        let portas = parse_ports("443, 22, 8000-8002, 22").unwrap();
        assert_eq!(portas, vec![22, 443, 8000, 8001, 8002]);
    }

    #[test]
    fn tcp_scan_concorrente_e_banner_grabbing() {
        let (addr, handle) = crate::tcp_server_spawn!("127.0.0.1:0", |cli| {
            let _ = cli.send(b"SSH-2.0-OpenSSH_9.6\r\n");
        });

        let porta_aberta = addr.port();
        let porta_fechada = if porta_aberta > 2000 {
            porta_aberta - 1
        } else {
            porta_aberta + 1
        };

        let resultados = tcp_scan(
            "127.0.0.1",
            &[porta_fechada, porta_aberta],
            0.5,
            2,
            true,
        );
        assert_eq!(resultados.len(), 2);

        let r_open = resultados.iter().find(|r| r.port == porta_aberta).unwrap();
        assert!(r_open.open);
        assert_eq!(r_open.banner.as_deref(), Some("SSH-2.0-OpenSSH_9.6"));
        handle.join().unwrap();
    }

    #[test]
    fn packet_crafting_ipv4_tcp_checksum_e_pcap_writer() {
        let src = Ipv4Addr::new(10, 0, 0, 1);
        let dst = Ipv4Addr::new(10, 0, 0, 2);
        let pkt = Ipv4TcpPacket::new(src, dst, 4444, 80, TCP_SYN | TCP_ACK)
            .with_seq(0x11223344)
            .with_ack(0x55667788)
            .with_payload(b"GET / HTTP/1.0\r\n\r\n");

        let raw = pkt.build();
        assert_eq!(rfc1071_checksum(&raw[0..20]), 0);
        assert_eq!(ipv4_tcp_checksum(src, dst, &raw[20..]), 0);

        let dissecado = Ipv4TcpPacket::parse(&raw).unwrap();
        assert_eq!(dissecado, pkt);

        let mut pcap = PcapWriter::new_raw_ip();
        pcap.add_packet(1_700_000_000, 500, &raw);
        let bytes_pcap = pcap.into_bytes();
        assert_eq!(bytes_pcap.len(), 24 + 16 + raw.len());
        assert_eq!(&bytes_pcap[0..4], &0xA1B2_C3D4u32.to_le_bytes());
    }

    #[test]
    fn proc_net_tcp_e_arp_parsers_funcionam() {
        let amostra_tcp = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:0035 00000000:0000 0A 00000000:00000000 00:00000000 00000000   101        0 20485 1 0000000000000000 100 0 0 10 0
   1: 6401A8C0:D431 0101A8C0:01BB 01 00000000:00000000 02:00000120 00000000  1000        0 44123 2 0000000000000000 20 4 30 10 -1\n";

        let entradas = parse_proc_net_tcp_str(amostra_tcp);
        assert_eq!(entradas.len(), 2);
        assert_eq!(entradas[0].local_addr.ip(), &Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(entradas[0].local_addr.port(), 53);
        assert_eq!(entradas[0].state, "LISTEN");
        assert_eq!(entradas[0].uid, 101);
        assert_eq!(entradas[0].inode, 20485);

        assert_eq!(entradas[1].local_addr.ip(), &Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(entradas[1].rem_addr.ip(), &Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(entradas[1].rem_addr.port(), 443);
        assert_eq!(entradas[1].state, "ESTABLISHED");

        let amostra_arp = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:ff     *        wlan0\n";
        let arp = parse_proc_net_arp_str(amostra_arp);
        assert_eq!(arp.len(), 1);
        assert_eq!(arp[0].ip, Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(arp[0].mac, "aa:bb:cc:dd:ee:ff");
        assert_eq!(arp[0].device, "wlan0");
    }

    #[test]
    fn sec_utils_pack_hex_base64_xor_entropia_e_cyclic() {
        assert_eq!(u64_le(&p64(0xDEAD_BEEF_CAFE_BABEu64)), Some(0xDEAD_BEEF_CAFE_BABEu64));
        assert_eq!(u32_be(&p32_be(0x1122_3344)), Some(0x1122_3344));

        let bytes = crate::unhex!("dead beef \\x90 0x41");
        assert_eq!(bytes, vec![0xDE, 0xAD, 0xBE, 0xEF, 0x90, 0x41]);
        assert_eq!(hex_encode(&bytes), "deadbeef9041");

        let b64 = base64_encode(b"RedTeam:PurpleTeam");
        assert_eq!(b64, "UmVkVGVhbTpQdXJwbGVUZWFt");
        assert_eq!(base64_decode(&b64).unwrap(), b"RedTeam:PurpleTeam");

        let claro = b"PAYLOAD_SECRETO_C2";
        let chave = b"K3Y";
        let cifrado = xor_cyclic(claro, chave);
        assert_ne!(&cifrado, claro);
        assert_eq!(xor_cyclic(&cifrado, chave), claro);

        assert!((shannon_entropy(&[0x90u8; 128]) - 0.0).abs() < 1e-9);
        let todos_256: Vec<u8> = (0u8..=255u8).collect();
        assert!((shannon_entropy(&todos_256) - 8.0).abs() < 1e-9);
        assert!(is_high_entropy(&todos_256, 7.9));

        let padrao = cyclic(512);
        let fatia_eip = &padrao[136..140];
        assert_eq!(cyclic_find(fatia_eip), Some(136));

        let dump = hexdump(b"Hello Rust!\n");
        assert!(dump.starts_with("00000000  48 65 6c 6c 6f 20 52 75  73 74 21 0a"));
        assert!(dump.ends_with("|Hello Rust!.|"));

        assert!(constant_time_eq(b"hmac_valido", b"hmac_valido"));
        assert!(!constant_time_eq(b"hmac_valido", b"hmac_falso!"));

        let mut segredo = vec![0xAA, 0xBB, 0xCC, 0xDD];
        secure_zero(&mut segredo);
        assert_eq!(segredo, vec![0, 0, 0, 0]);
    }
}

#[cfg(test)]
mod tests_v151_sorted {
    use super::prelude::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn dict_basico() {
        let mut d: BTreeMap<&str, i32> = sorted_dict! { "b" => 2, "a" => 1, "c" => 3 };
        assert_eq!(d.get_or("z", 0), 0);
        assert_eq!(d.get_or("a", 0), 1);
        *d.setdefault("d", 0) += 5;
        assert_eq!(d["d"], 5);
        assert_eq!(d.pop("a"), Some(1));
        assert_eq!(d.pop("a"), None);
        d.update(vec![("e", 9), ("b", 20)]);
        assert_eq!(d["b"], 20);
        assert_eq!(d.first_key(), Some(&"b"));
        assert_eq!(d.last_key(), Some(&"e"));
        assert_eq!(d.first_item(), Some((&"b", &20)));
        assert_eq!(d.last_item(), Some((&"e", &9)));
        assert_eq!(d.nth_item(1), Some((&"c", &3)));
        let vazio: BTreeMap<i32, i32> = sorted_dict! {};
        assert!(vazio.first_key().is_none());
    }

    #[test]
    fn dict_busca_ordenada() {
        let d: BTreeMap<i32, char> = sorted_dict! { 10 => 'a', 20 => 'b', 30 => 'c' };
        assert_eq!(d.floor_key(&25), Some(&20));
        assert_eq!(d.floor_key(&20), Some(&20));
        assert_eq!(d.floor_key(&5), None);
        assert_eq!(d.ceiling_key(&25), Some(&30));
        assert_eq!(d.ceiling_key(&20), Some(&20));
        assert_eq!(d.ceiling_key(&31), None);
        assert_eq!(d.lower_key(&20), Some(&10));
        assert_eq!(d.higher_key(&20), Some(&30));
        assert_eq!(d.bisect_left(&20), 1);
        assert_eq!(d.bisect_right(&20), 2);
        assert_eq!(d.irange(&10, &20), vec![(&10, &'a'), (&20, &'b')]);
        assert!(d.irange(&30, &10).is_empty());
    }

    #[test]
    fn set_basico() {
        let mut s: BTreeSet<i32> = sorted_set![5, 1, 3];
        s.add(4);
        s.discard(&1);
        s.discard(&99);
        s.update(vec![8, 2]);
        assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4, 5, 8]);
        assert_eq!(s.nth(0), Some(&2));
        assert_eq!(s.floor(&6), Some(&5));
        assert_eq!(s.ceiling(&6), Some(&8));
        assert_eq!(s.lower(&3), Some(&2));
        assert_eq!(s.higher(&8), None);
        assert_eq!(s.bisect_left(&4), 2);
        assert_eq!(s.bisect_right(&4), 3);
        assert_eq!(s.irange(&3, &5), vec![&3, &4, &5]);
        assert!(s.irange(&5, &3).is_empty());
    }

    #[test]
    fn set_com_str_emprestado() {
        let mut s: BTreeSet<String> = sorted_set!["b".to_string(), "a".to_string()];
        assert_eq!(s.floor("az"), Some(&"a".to_string()));
        s.discard("a");
        assert_eq!(s.len(), 1);
    }
}

// ============================================================================
// Testes v2.0.0 — Correções Críticas, Novos Módulos e Casos Extremos
// ============================================================================

#[cfg(test)]
mod tests_v2_range_list_collections_iter {
    use super::collection_utils::*;
    use super::iterator_utils::*;
    use super::list_utils::*;
    use super::range_utils::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    #[test]
    fn range_extremos_sem_overflow_e_double_ended() {
        // Sem overflow em next() no limite de i64::MAX e i64::MIN
        let mut r_max = range_step(i64::MAX - 2, i64::MAX, 5);
        assert_eq!(r_max.next(), Some(i64::MAX - 2));
        assert_eq!(r_max.next(), None);
        assert_eq!(r_max.next(), None);

        let mut r_min = range_step(i64::MIN + 2, i64::MIN, -5);
        assert_eq!(r_min.next(), Some(i64::MIN + 2));
        assert_eq!(r_min.next(), None);

        let full = range_from(i64::MIN, i64::MAX);
        assert_eq!(full.len_u128(), (u64::MAX as u128));
        assert!(full.contains(0));
        assert!(full.contains(i64::MIN));
        assert!(!full.contains(i64::MAX));

        // DoubleEndedIterator
        let rev_v: Vec<i64> = range_step(0, 10, 2).rev().collect();
        assert_eq!(rev_v, vec![8, 6, 4, 2, 0]);
        let rev_neg: Vec<i64> = range_step(10, 1, -3).rev().collect();
        assert_eq!(rev_neg, vec![4, 7, 10]);

        let r = range_step(10, 20, 3);
        assert_eq!(r.get(1), Some(13));
        assert_eq!(r.index_of(16), Some(2));
        assert_eq!(r.index_of(17), None);
        assert_eq!(try_range_step(0, 5, 0), Err(RangeError::StepZero));
        assert_eq!(range_idx(4).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        assert_eq!(range_idx_from(5, 3).collect::<Vec<_>>(), Vec::<usize>::new());
    }

    #[test]
    fn pylist_indices_negativos_slicing_e_dedup() {
        let mut v = vec![10, 20, 30, 20, 40];
        assert_eq!(v.get_py(-1), Some(&40));
        assert_eq!(v.get_py(-10), None);
        *v.get_py_mut(-2).unwrap() = 25;
        assert_eq!(v.slice_py(Some(1), Some(-1)), &[20, 30, 25]);
        assert_eq!(v.slice_step(None, None, -2), vec![40, 30, 10]);

        v.insert_py(-1, 99);
        assert_eq!(v[4], 99);
        assert_eq!(v.pop_py(-2), Some(99));
        v.append_item(10);
        v.dedup_preserve_order();
        assert_eq!(v, vec![10, 20, 30, 25, 40]);

        let mut d = vec![1, 2, 1, 3, 1];
        assert_eq!(d.remove_all(&1), 3);
        assert_eq!(d, vec![2, 3]);
        assert_eq!(d.copy_list(), vec![2, 3]);
        d.sort_desc();
        assert_eq!(d, vec![3, 2]);
        assert!(d.swap_remove_value(&3));
        assert_eq!(d.index_in(&2, 0, 1), Some(0));
    }

    #[test]
    fn colecoes_dict_set_deque_heaps_e_macros() {
        let mut d: HashMap<&str, i32> = crate::dict! { "a" => 1, "b" => 2 };
        assert_eq!(d.get_or("z", 9), 9);
        assert_eq!(*d.get_or_ref("a", &0), 1);
        assert_eq!(d.get_or_else("z", || 77), 77);
        *d.setdefault_with("c", || 10) += 5;
        assert_eq!(d["c"], 15);
        assert_eq!(d.pop_or("missing", -1), -1);
        d.modify_or_insert("a", 0, |x| *x += 9);
        assert_eq!(d["a"], 10);

        let d2 = crate::dict! { "b" => 200, "d" => 4 };
        let merged = d.merge_cloned(&d2);
        assert_eq!(merged["b"], 200);
        assert_eq!(merged["a"], 10);

        let mut s1: HashSet<i32> = crate::set![1, 2, 3];
        let s2: HashSet<i32> = crate::set![2, 3, 4];
        assert!(s1.add(5));
        assert!(!s1.add(5));
        assert!(s1.discard(&5));
        assert!(s1.remove_strict(&99).is_err());
        assert_eq!(s1.union_set(&s2), crate::set![1, 2, 3, 4]);
        assert_eq!(s1.intersection_set(&s2), crate::set![2, 3]);
        assert_eq!(s1.difference_set(&s2), crate::set![1]);
        assert_eq!(s1.symmetric_difference_set(&s2), crate::set![1, 4]);

        let mut dq: VecDeque<i32> = crate::deque![1, 2, 3];
        dq.appendleft(0);
        dq.append_item(4);
        dq.rotate_py(1);
        assert_eq!(dq.peek_left(), Some(&4));
        assert_eq!(dq.popleft(), Some(4));
        assert_eq!(dq.pop_right(), Some(3));

        let mut mh = MinHeap::from_vec(vec![5, 1, 4, 2]);
        assert_eq!(mh.peek(), Some(&1));
        assert_eq!(mh.push_pop(3), 1);
        assert_eq!(mh.into_sorted_vec(), vec![2, 3, 4, 5]);

        let mut pq = PriorityQueue::max_heap();
        pq.push(10);
        pq.push(30);
        pq.push(20);
        assert_eq!(pq.into_sorted_vec(), vec![30, 20, 10]);
    }

    #[test]
    fn iteradores_lazy_completos() {
        let z: Vec<_> = zip_longest(vec![1, 2], vec!["a", "b", "c"]).collect();
        assert_eq!(
            z,
            vec![
                EitherOrBoth::Both(1, "a"),
                EitherOrBoth::Both(2, "b"),
                EitherOrBoth::Right("c")
            ]
        );

        let lotes: Vec<_> = batched(0..5, 2).collect();
        assert_eq!(lotes, vec![vec![0, 1], vec![2, 3], vec![4]]);

        let sl: Vec<_> = islice(0..10, 1, Some(8), 3).collect();
        assert_eq!(sl, vec![1, 4, 7]);

        let cnt: Vec<_> = count_from(i64::MAX - 2, 2).collect();
        assert_eq!(cnt, vec![i64::MAX - 2, i64::MAX]);

        let (pares, impares) = partition(1..=6, |x| x % 2 == 0);
        assert_eq!(pares, vec![2, 4, 6]);
        assert_eq!(impares, vec![1, 3, 5]);

        assert_eq!(reduce(vec![1, 2, 3, 4], |a, b| a + b), Some(10));
        assert_eq!(fold(vec![1, 2, 3], 10, |acc, x| acc + x), 16);

        let freq = "aba".chars().frequencies();
        assert_eq!(freq.get(&'a'), Some(&2));
        let enum_s: Vec<_> = vec!["x", "y"].into_iter().enumerate_from(10).collect_vec();
        assert_eq!(enum_s, vec![(10, "x"), (11, "y")]);
        assert!([false, true].into_iter().any_true());
        assert!([true, true].into_iter().all_true());
    }
}

#[cfg(test)]
mod tests_v2_matrix_builtins_arith_str_class {
    use super::arithmetic::*;
    use super::builtins::*;
    use super::matrix_utils::*;
    use super::string_utils::*;

    #[test]
    fn matrix_edge_cases_e_operacoes() {
        assert_eq!(
            Matrix::try_new(usize::MAX, 2, 0u8),
            Err(MatrixError::OverflowDeDimensao)
        );
        // Matriz com 0 colunas não pode dar panic em rows()
        let zc = Matrix::new(3, 0, 0i32);
        assert_eq!(zc.rows().count(), 3);
        assert_eq!(len(&zc), 3);

        let m = Matrix::from_rows(&[&[1, 2], &[3, 4]]).unwrap();
        let c1: Vec<i32> = m.col(1).unwrap().copied().collect();
        assert_eq!(c1, vec![2, 4]);
        assert_eq!(m.trace(), Ok(5));
        assert_eq!(Matrix::<i32>::identity(2).trace(), Ok(2));

        let m2 = Matrix::from_vec(2, 2, vec![10, 20, 30, 40]).unwrap();
        assert_eq!(m.add(&m2).unwrap().as_slice(), &[11, 22, 33, 44]);
        assert_eq!(m.hadamard(&m2).unwrap().as_slice(), &[10, 40, 90, 160]);
        assert_eq!(m.scale(2).as_slice(), &[2, 4, 6, 8]);
        assert!(Matrix::<i32>::zeros(2, 3).try_dot(&Matrix::zeros(4, 2)).is_err());
    }

    #[test]
    fn builtins_nan_e_totais() {
        // Consistente independente da posição do NaN
        assert_eq!(min(vec![f64::NAN, 2.0, 1.0]), Some(1.0));
        assert_eq!(min(vec![2.0, f64::NAN, 1.0]), Some(1.0));
        assert_eq!(max(vec![f64::NAN, 2.0, 5.0]), Some(5.0));
        assert_eq!(min(vec![f64::NAN, f64::NAN]), None);

        assert_eq!(try_min(vec![1.0, f64::NAN]), Err(CompareError::Incomparable));
        assert_eq!(try_max(vec![f64::NAN, 1.0]), Err(CompareError::Incomparable));
        assert_eq!(try_sorted(vec![f64::NAN]), Err(CompareError::Incomparable));
        assert_eq!(try_sorted(vec![3.0, 1.0, 2.0]), Ok(vec![1.0, 2.0, 3.0]));

        let tot = sorted_f64_total(vec![f64::NAN, 3.0, -0.0, 1.0]);
        assert_eq!(tot[0], -0.0);
        assert!(tot[3].is_nan());

        assert_eq!(sorted_desc(vec![1, 3, 2]), vec![3, 2, 1]);
        assert_eq!(sorted_by_key(vec!["bbb", "a", "cc"], |s| s.len()), vec!["a", "cc", "bbb"]);
        assert_eq!(clamp(15, 0, 10), 10);
        assert_eq!(try_clamp(f64::NAN, 0.0, 1.0), Err(CompareError::Incomparable));
        assert_eq!(product(vec![2, 3, 5]), 30);
        assert!(contains_in(&[1, 2, 3], &2));
        assert_eq!(count_in(&[1, 2, 1], &1), 2);
        assert_eq!(find_in(&[10, 20, 30], &20), Some(1));
    }

    #[test]
    fn arithmetic_segura_floored_e_extremos() {
        assert_eq!(div(i64::MIN, -1), Err(ArithError::Overflow));
        assert_eq!(divmod(i64::MIN, -1), Err(ArithError::Overflow));
        assert_eq!(checked_abs(i64::MIN), Err(ArithError::Overflow));
        assert!(abs(-0.0f64).is_sign_positive());

        assert_eq!(root(-8.0, 1e20), Err(ArithError::RaizDeNegativoComIndicePar));
        assert_eq!(root(0.0, -2.0), Err(ArithError::DivisaoPorZero));
        assert_eq!(root(f64::NAN, 3.0), Err(ArithError::DominioInvalido));
        assert_eq!(root(9.0, f64::INFINITY), Err(ArithError::IndiceDeRaizInvalido));

        assert_eq!(py_divmod(-7i64, 3), Ok((-3, 2)));
        assert_eq!(py_divmod(7i64, -3), Ok((-3, -2)));
        assert_eq!(floor_div(-7i64, 3), Ok(-3));
        assert_eq!(ceil_div(7i64, 3), Ok(3));
        assert_eq!(ceil_div(-7i64, 3), Ok(-2));

        assert_eq!(checked_sum(vec![i8::MAX, 1]), Err(ArithError::Overflow));
        assert_eq!(saturating_add(250u8, 20), 255);
        assert_eq!(wrapping_add(255u8, 2), 1);
        assert_eq!(gcd(48i64, -18), 6);
        assert_eq!(checked_gcd(i64::MIN, 0), Err(ArithError::Overflow));
        assert_eq!(lcm(12i64, 18), 36);
        assert!(is_close(0.1 + 0.2, 0.3, 1e-9, 0.0));
        assert_eq!(try_cast::<i32, u8>(300), Err(ArithError::ConversaoInvalida));
        assert_eq!(
            f64_to_i64_checked(9223372036854775808.0),
            Err(ArithError::ConversaoInvalida)
        );
        assert_eq!(f64_to_i64_checked(-42.9), Ok(-42));
    }

    #[test]
    fn strings_unicode_zero_copy_e_padding() {
        assert_eq!(partition("chave=val=2", "="), ("chave", "=", "val=2"));
        assert_eq!(rpartition("chave=val=2", "="), ("chave=val", "=", "2"));
        assert_eq!(removeprefix("pyrsutils", "py"), "rsutils");
        assert_eq!(removesuffix("pyrsutils", "utils"), "pyrs");
        assert_eq!(strip_chars("__oi__", &['_']), "oi");
        assert_eq!(substring("olá_🦀_rust", 4, 5), "🦀");
        assert_eq!(substring_py("olá_🦀_rust", Some(-4), None), "rust");
        assert!(!is_numeric(""));
        assert!(is_numeric("123²"));
        assert!(!is_ascii_digits("123²"));
        assert!(is_ascii_digits("123"));
        assert_eq!(pad_left("7", 3, '0'), "007");
        assert_eq!(pad_right("7", 3, '0'), "700");
        assert_eq!(center("hi", 6, '-'), "--hi--");
    }

    crate::class! {
        /// Exemplo com atributos e `as_base`.
        struct BaseItem {
            id: u32
        }
        impl {
            fn get_id(&self) -> u32 { self.id }
        }
    }

    crate::class! {
        struct NamedItem : BaseItem {
            name: String
        }
        impl {
            fn label(&self) -> String { format!("{}:{}", self.get_id(), self.name) }
        }
    }

    #[test]
    fn class_atributos_e_as_base() {
        let item = NamedItem::new(BaseItem::new(7), "ok".to_string());
        assert_eq!(item.as_base().get_id(), 7);
        assert_eq!(item.label(), "7:ok");
    }
}

#[cfg(test)]
mod tests_v2_input_time_fs_io_env_proc_term_conc {
    use super::concurrency_utils::*;
    use super::env_utils::*;
    use super::fs_utils::*;
    use super::input_utils::*;
    use super::io_utils::*;
    use super::terminal_utils::*;
    use super::time_utils::*;
    use std::io::Cursor;

    #[test]
    fn input_limitado_e_bufread() {
        let dados = b"  42  \r\nol\xC3\xA1 mundo\n";
        let mut r = std::io::BufReader::new(Cursor::new(&dados[..]));
        assert_eq!(read_line_from_limited(&mut r, 32).unwrap(), Some("  42  ".to_string()));
        assert_eq!(read_line_from_limited(&mut r, 32).unwrap(), Some("olá mundo".to_string()));
        assert_eq!(read_line_from_limited(&mut r, 32).unwrap(), None);

        let mut longo = std::io::BufReader::new(Cursor::new(b"abcdef\n"));
        assert!(read_line_from_limited(&mut longo, 4).is_err());

        let mut r_num = std::io::BufReader::new(Cursor::new(b" 123 \n"));
        let val: i32 = try_input_from(&mut r_num).unwrap();
        assert_eq!(val, 123);
    }

    #[test]
    fn tempo_timer_deadline_e_validacao_utc() {
        assert_eq!(try_sleep(-0.5), Err(TimeError::InvalidDuration));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));

        assert!(StructTime::new_utc(2023, 2, 29, 0, 0, 0).is_err());
        let st = StructTime::new_utc(2024, 2, 29, 12, 30, 45).unwrap();
        assert_eq!(st.to_iso8601(), "2024-02-29T12:30:45Z");
        let ts = try_mktime_utc(&st).unwrap();
        assert_eq!(gmtime(ts), st);

        // Extremos sem panic
        let _ = gmtime(i64::MIN);
        let _ = gmtime(i64::MAX);
        let _ = gmtime_now();

        let invalido = StructTime {
            year: 2024,
            month: 0,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            weekday: 99,
            yearday: 1,
        };
        assert_eq!(strftime("%a %b", &invalido), "??? ???");
        assert!(try_mktime_utc(&invalido).is_err());

        let mut tm = Timer::start();
        let _ = tm.lap();
        assert!(tm.elapsed_ns() < 10_000_000_000);
        let dl = Deadline::from_secs(1.0).unwrap();
        assert!(!dl.is_expired());
    }

    #[test]
    fn fs_glob_patologico_atomic_copy_tree_e_io() {
        let base = std::env::temp_dir().join(format!("pyrsutils_v2_io_{}", std::process::id()));
        let _ = rmtree(&base);
        makedirs(base.join("sub")).unwrap();

        let patologico_nome = "aaaaaaaaaaaaaaaaaaaaaaaaaac.txt";
        write_text_atomic(base.join(patologico_nome), "l1\nl2\nl3\n").unwrap();
        write_text_atomic(base.join("sub/x.txt"), "ok").unwrap();

        // Glob com padrão patológico que derrubaria recursão ingênua O(2^N)
        let padrao = format!(
            "{}/*a*a*a*a*a*a*a*a*a*a*a*a*b",
            base.to_string_lossy()
        );
        let achou = glob(&padrao).unwrap();
        assert!(achou.is_empty());

        let arq = base.join(patologico_nome);
        assert!(read_text_limited(&arq, 3).is_err());
        assert_eq!(read_text_limited(&arq, 64).unwrap(), "l1\nl2\nl3\n");

        let linhas: Vec<_> = read_lines_lazy(&arq).unwrap().collect::<Result<_, _>>().unwrap();
        assert_eq!(linhas, vec!["l1", "l2", "l3"]);

        let mut soma_linhas = 0usize;
        for_each_line(&arq, |idx, linha| {
            soma_linhas += idx + linha.len();
            Ok(())
        })
        .unwrap();
        assert_eq!(soma_linhas, (0 + 2) + (1 + 2) + (2 + 2));

        let dst = base.join("copia");
        assert_eq!(copy_tree(base.join("sub"), &dst).unwrap(), 1);
        assert_eq!(read_text(dst.join("x.txt")).unwrap(), "ok");
        assert!(metadata(dst.join("x.txt")).unwrap().is_file);
        assert_eq!(splitext("/"), ("/".to_string(), "".to_string()));

        let mut src_cur = Cursor::new(b"123456".to_vec());
        let mut dst_vec = Vec::new();
        assert!(copy_stream_limited(&mut src_cur, &mut dst_vec, 4).is_err());

        let buf_arq = base.join("buf.txt");
        {
            let mut bw = BufferedWriter::create(&buf_arq).unwrap();
            bw.write_line("alpha").unwrap();
            bw.write_line("beta").unwrap();
            bw.flush().unwrap();
        }
        let mut br = BufferedReader::open(&buf_arq).unwrap();
        assert_eq!(br.read_line_trimmed().unwrap(), Some("alpha".to_string()));
        assert_eq!(br.read_line_trimmed().unwrap(), Some("beta".to_string()));
        assert_eq!(br.read_line_trimmed().unwrap(), None);

        rmtree(&base).unwrap();
    }

    #[test]
    fn env_terminal_e_concorrencia() {
        assert!(!os_name().is_empty());
        assert!(!arch().is_empty());
        assert!(available_parallelism() >= 1);
        assert_eq!(var_or("__PYRSUTILS_VAR_INEXISTENTE__", "padrao"), "padrao");

        let ansi = colorize_always("alerta", Color::Red);
        assert!(ansi.contains("\x1b[31m"));
        assert_eq!(strip_ansi(&ansi), "alerta");

        let mut pb = ProgressBar::new(10, 10);
        pb.set_color_mode(ColorMode::Never);
        pb.update(5);
        assert_eq!(pb.render(), "[#####-----]  50.0% (5/10)");

        let mut out = Vec::new();
        let mut inp = Cursor::new(b"invalid\ns\n");
        assert!(confirm_with(&mut inp, &mut out, "Continuar", Some(false)).unwrap());

        let mut out2 = Vec::new();
        let mut inp2 = Cursor::new(b"2\n");
        assert_eq!(
            select_menu_with(&mut inp2, &mut out2, "Escolha", &["um", "dois"]).unwrap(),
            1
        );

        let res = parallel_map((0..20).collect(), 4, |x| x * x);
        assert_eq!(res, (0..20).map(|x| x * x).collect::<Vec<_>>());
        assert_eq!(parallel_map(vec![7], 4, |x| x + 1), vec![8]);

        let sh = Shared::new(0usize);
        let items: Vec<usize> = (1..=10).collect();
        parallel_for_each(&items, 4, |&x| {
            sh.try_with_mut(|acc| *acc += x).unwrap();
        });
        assert_eq!(sh.get_cloned().unwrap(), 55);

        let pool = ThreadPool::new(2);
        let rx = pool.submit(|| 21 * 2).unwrap();
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[cfg(unix)]
    #[test]
    fn process_utils_unix() {
        use super::process_utils::*;
        let out = run_checked("echo", ["ola_proc"]).unwrap();
        assert_eq!(out.stdout_text().trim(), "ola_proc");

        assert!(matches!(
            run_checked("false", [] as [&str; 0]),
            Err(ProcessError::NonZeroExit { .. })
        ));

        let cat_out = run_with_stdin("cat", [] as [&str; 0], b"via_stdin").unwrap();
        assert!(cat_out.success());
        assert_eq!(&cat_out.stdout, b"via_stdin");

        let pipe_out = run_pipeline(&[("echo", &["pipe_ok"]), ("cat", &[])]).unwrap();
        assert!(pipe_out.success());
        assert_eq!(pipe_out.stdout_text().trim(), "pipe_ok");
    }
}

#[cfg(test)]
mod tests_v2_net_random_sec_sorted_sql_prelude {
    use super::net_utils::*;
    use super::random_utils::*;
    use super::sec_utils::*;
    use super::sorted_utils::*;
    use super::sql_utils::*;
    use std::collections::{BTreeMap, BTreeSet, HashSet};
    use std::io::Cursor;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn net_ipv6_range_tcp_udp_pcap_e_scanner() {
        let v6_zero = parse_cidr_v6("::/0").unwrap();
        assert!(v6_zero.contains(Ipv6Addr::LOCALHOST));
        assert_eq!(v6_zero.total_addresses(), None);

        let v6 = parse_cidr_v6("2001:db8::/126").unwrap();
        assert_eq!(v6.total_addresses(), Some(4));
        assert_eq!(v6.count(), 4);

        let ipr = Ipv4Range::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 3)).unwrap();
        assert!(ipr.contains(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(ipr.count(), 3);

        let ip_uni = IpCidr::parse("10.0.0.0/8").unwrap();
        assert!(ip_uni.contains(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));

        // Ipv4TcpPacket payload > 65495 deve falhar limpo em try_build
        let pkt_gigante = Ipv4TcpPacket::new(
            Ipv4Addr::LOCALHOST,
            Ipv4Addr::LOCALHOST,
            1,
            2,
            TCP_SYN,
        )
        .with_payload(vec![0u8; 65500]);
        assert_eq!(pkt_gigante.try_build(), Err(NetPacketError::PayloadTooLarge));

        // Ethernet padding extra não deve entrar no payload após parse()
        let pkt = Ipv4TcpPacket::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 2),
            1234,
            80,
            TCP_ACK,
        )
        .with_payload(b"HI");
        let mut raw_com_padding = pkt.build();
        raw_com_padding.extend_from_slice(&[0u8; 18]);
        assert_eq!(Ipv4TcpPacket::parse(&raw_com_padding).unwrap(), pkt);
        assert!(Ipv4TcpPacket::parse_checked(&raw_com_padding).is_ok());

        // Checksum corrompido deve falhar em parse_checked
        let mut raw_corrompido = raw_com_padding.clone();
        raw_corrompido[10] ^= 0xFF;
        assert_eq!(
            Ipv4TcpPacket::parse_checked(&raw_corrompido),
            Err(NetPacketError::ChecksumMismatch)
        );

        let udp = Ipv4UdpPacket::new(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 5000, 5001)
            .with_payload(b"dgram");
        let udp_raw = udp.build();
        assert_eq!(Ipv4UdpPacket::parse_checked(&udp_raw).unwrap(), udp);

        // StreamingPcapWriter + PcapReader round-trip
        let mut pcap_bytes = Vec::new();
        {
            let mut sw = StreamingPcapWriter::new_raw_ip(&mut pcap_bytes).unwrap();
            sw.write_packet(100, 200, &udp_raw).unwrap();
            sw.flush().unwrap();
        }
        let mut pr = PcapReader::new(Cursor::new(&pcap_bytes)).unwrap();
        let rec = pr.next_packet_limited(2048).unwrap().unwrap();
        assert_eq!(rec.data, udp_raw);
        assert!(pr.next_packet_limited(2048).unwrap().is_none());

        let mut pr_cap = PcapReader::new(Cursor::new(&pcap_bytes)).unwrap();
        assert!(pr_cap.next_packet_limited(4).is_err());

        // Scanner detalhado em loopback
        let (addr, handle) = crate::tcp_server_spawn!("127.0.0.1:0", |cli| {
            let _ = cli.send(b"BANNER\n");
        });
        let aberta = addr.port();
        let fechada = if aberta > 2000 { aberta - 1 } else { aberta + 1 };
        let cfg = ScanConfig {
            timeout_secs: 0.5,
            threads: 4,
            grab_banner: true,
            banner_max_bytes: 64,
            rate_limit_per_sec: None,
        };
        let res = tcp_scan_with_config("127.0.0.1", &[fechada, aberta], &cfg);
        assert_eq!(
            res.iter().find(|r| r.port == aberta).unwrap().state,
            PortState::Open
        );
        assert_eq!(
            res.iter().find(|r| r.port == fechada).unwrap().state,
            PortState::Closed
        );
        handle.join().unwrap();
    }

    #[test]
    fn net_tcp_peek_recvuntil_strict_e_udp_connect() {
        let (addr, handle) = crate::tcp_server_spawn!("127.0.0.1:0", |cli| {
            cli.send(b"HELLO:WORLD").unwrap();
        });
        let mut cli = TcpClient::connect(addr).unwrap();
        cli.set_timeout(Some(1.0)).unwrap();
        let peeked = cli.peek(5).unwrap();
        assert_eq!(&peeked, b"HELLO");
        let ate = cli.recvuntil_strict(b":", 64).unwrap();
        assert_eq!(&ate, b"HELLO:");
        assert!(cli.recvuntil_strict(b"MISSING", 64).is_err());
        handle.join().unwrap();

        let srv = UdpSock::bind("127.0.0.1:0").unwrap();
        let cli_u = UdpSock::bind("127.0.0.1:0").unwrap();
        cli_u.connect(srv.local_addr().unwrap()).unwrap();
        cli_u.send(b"connected_udp").unwrap();
        assert_eq!(srv.recv_from(64).unwrap().0, b"connected_udp");
    }

    #[test]
    fn random_pyrng_unbiased_e_os_random() {
        let mut r1 = PyRng::from_seed(12345);
        let mut r2 = PyRng::from_seed(12345);
        for _ in 0..20 {
            assert_eq!(r1.next_u64(), r2.next_u64());
        }
        assert!(try_randint(10, 5).is_err());
        let _ = randint(i64::MIN, i64::MAX);
        assert!(r1.try_randint(5, 1).is_err());
        let itens = [1, 2, 3, 4, 5];
        assert_eq!(r1.sample(&itens, 3).unwrap().len(), 3);
        assert!(r1.sample(&itens, 10).is_err());
        assert_eq!(randbytes_insecure(12).len(), 12);

        #[cfg(unix)]
        {
            let cs = os_random_bytes(32).unwrap();
            assert_eq!(cs.len(), 32);
        }
    }

    #[test]
    fn sec_debruijn_base64_estrito_binreader_e_crc32() {
        // Verifica unicidade de TODAS as janelas de 4 bytes no padrão De Bruijn
        let padrao = cyclic(4096);
        assert_eq!(padrao.len(), 4096);
        let mut janelas = HashSet::new();
        for w in padrao.windows(4) {
            assert!(janelas.insert(w), "janela De Bruijn duplicada!");
        }
        // Busca em offset > 20.000 (superando o limite antigo de 20k!)
        let grande = cyclic(25_100);
        let fatia = &grande[25_000..25_004];
        assert_eq!(cyclic_find(fatia), Some(25_000));

        assert_eq!(de_bruijn(b"01", 3, 8).len(), 8);

        // Base64 RFC 4648 estrito
        assert!(base64_decode("AB=C").is_err());
        assert!(base64_decode("AA==AA==").is_err());
        assert!(base64_decode("SGVsbG9=").is_err());
        assert!(base64_decode_strict("SGVs bG8=").is_err());

        let u_enc = base64url_encode(b"\xfb\xff\xef", false);
        assert!(!u_enc.contains('+'));
        assert!(!u_enc.contains('/'));
        assert_eq!(base64url_decode(&u_enc).unwrap(), b"\xfb\xff\xef");
        assert!(base64url_decode("AA+B").is_err());

        // BinWriter & BinReader
        let mut bw = BinWriter::new(Endian::Big);
        bw.write_u32(0x1122_3344);
        bw.write_i16(-1234);
        bw.write_padding_align(8, 0xCC).unwrap();
        bw.write_cstr(b"ELF");
        bw.write_f64(3.5);
        let buf = bw.into_bytes();

        let mut br = BinReader::new(&buf, Endian::Big);
        assert_eq!(br.read_u32().unwrap(), 0x1122_3344);
        assert_eq!(br.read_i16().unwrap(), -1234);
        br.align(8).unwrap();
        assert_eq!(br.read_cstr_utf8().unwrap(), "ELF");
        assert!((br.read_f64().unwrap() - 3.5).abs() < 1e-12);
        assert!(br.read_u8().is_err());

        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_ne!(fnv1a_64(b"abc"), 0);

        let aob = parse_aob_pattern("DE ?? BE EF").unwrap();
        assert_eq!(
            find_wildcard_pattern(&[0x00, 0xDE, 0xAA, 0xBE, 0xEF], &aob),
            Some(1)
        );
        assert_eq!(
            extract_ascii_strings(b"\x00\x01hello\x00world!\x02", 5),
            vec!["hello", "world!"]
        );
    }

    #[test]
    fn sorted_e_sql_utils() {
        let mut sd: BTreeMap<i32, &str> = crate::sorted_dict! { 10 => "a", 20 => "b", 30 => "c", 40 => "d" };
        assert_eq!(sd.get_or_else(&99, || "fallback"), "fallback");
        assert_eq!(sd.nth_item(3), Some((&40, &"d")));
        assert_eq!(sd.irange_iter(&15, &35).count(), 2);
        assert_eq!(sd.irange_iter(&35, &15).count(), 0);
        sd.pop(&10);

        let ss: BTreeSet<i32> = crate::sorted_set![1, 2, 3, 4, 5];
        assert_eq!(ss.nth(4), Some(&5));
        assert_eq!(ss.irange_iter(&2, &4).copied().collect::<Vec<_>>(), vec![2, 3, 4]);

        let tabela = SqlIdent::new("users").unwrap();
        let col_id = SqlIdent::new("id").unwrap();
        let col_role = SqlIdent::new("role").unwrap();
        assert!(SqlIdent::new("users; DROP TABLE users").is_err());

        let mut qb = SqlBuilder::new(SqlDialect::Postgres);
        qb.push_sql("SELECT * FROM ")
            .push_ident(&tabela)
            .where_eq(&col_id, 42i64)
            .and_eq(&col_role, "admin");
        let q = qb.build();
        assert_eq!(q.sql, "SELECT * FROM \"users\" WHERE \"id\" = $1 AND \"role\" = $2");
        assert_eq!(q.params, vec![SqlParam::Int(42), SqlParam::Text("admin".to_string())]);

        let tpl = SqlTemplate::new(
            "SELECT ':nao_trocar', id::int FROM items WHERE name = :nome AND active = :ok",
        )
        .unwrap();
        let rendered = tpl
            .render(
                SqlDialect::SqlServer,
                &[("nome", "a".into()), ("ok", true.into())],
            )
            .unwrap();
        assert_eq!(
            rendered.sql,
            "SELECT ':nao_trocar', id::int FROM items WHERE name = @p1 AND active = @p2"
        );
    }

    #[test]
    fn prelude_completo_sem_colisoes() {
        use super::prelude::*;
        use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

        let mut v = vec![1, 2];
        v.append_item(3);
        assert_eq!(len(&v), 3);

        let mut hm: HashMap<&str, i32> = crate::dict! { "k" => 1 };
        assert_eq!(hm.get_or("k", 0), 1);

        let mut hs: HashSet<i32> = crate::set![1, 2];
        hs.add(3);
        assert_eq!(len(&hs), 3);

        let mut dq: VecDeque<i32> = crate::deque![10, 20];
        dq.appendleft(5);
        assert_eq!(dq.popleft(), Some(5));

        let mut bm: BTreeMap<i32, i32> = crate::sorted_dict! { 1 => 10 };
        assert_eq!(bm.first_key(), Some(&1));

        let mut bs: BTreeSet<i32> = crate::sorted_set![1, 2];
        assert_eq!(bs.nth(1), Some(&2));

        assert_eq!(checked_add(2i32, 3), Ok(5));
        assert_eq!(floor_div(-7i64, 3), Ok(-3));
        assert_eq!(2i32.py_checked_mul(4), Ok(8));
        assert_eq!(3.2f64.py_ceil(), 4.0);
        assert_eq!(vec![1, 2, 3].into_iter().batched(2).collect_vec().len(), 2);
    }
}
