# Ultimate Parser — Backend

API REST construida en **Rust + Axum** que implementa algoritmos de parsing para el curso de Compiladores en UTEC. Expone endpoints para analizar gramáticas formales y simular el proceso de parsing paso a paso.

## Stack

| Tecnología | Versión | Rol |
|---|---|---|
| Rust | edition 2024 | Lenguaje base |
| Axum | 0.8 | Framework HTTP async |
| Tokio | 1.52 | Runtime async |
| Serde / serde_json | 1.0 | Serialización JSON |
| tower-http | 0.5 | Middleware CORS |

## Estructura del proyecto

```
src/
├── main.rs                  # Punto de entrada, configuración del servidor y rutas
├── api/
│   └── mod.rs               # Handlers HTTP (deserialización, orquestación, respuesta)
├── core/
│   ├── mod.rs               # Re-exports del módulo core
│   ├── models.rs            # Tipos de datos: Symbol, Production, Grammar, ParseSnapshot
│   ├── grammar.rs           # Lógica de parsing de gramáticas desde texto plano
│   └── tests/
│       ├── mod.rs
│       └── models_test.rs
└── parsers/
    ├── mod.rs               # Trait Parser + re-exports
    ├── ll1.rs               # Parser LL(1) completo (FIRST, FOLLOW, tabla, simulación)
    ├── lr0.rs               # Parser LR(0) (estructura de ítems y closure)
    ├── recursive_descent.rs # Placeholder para descenso recursivo (Fase 4)
    └── tests/
        ├── mod.rs
        ├── ll1_test.rs
        └── lr0_test.rs
```

## Ejecutar el servidor

```bash
# Desde la raíz del backend
cargo run

# El servidor queda escuchando en:
# http://127.0.0.1:3000
```

```bash
# Correr tests
cargo test
```

## Endpoints

### `GET /`

Healthcheck. Retorna un string indicando que la API está activa.

---

### `POST /analyze-grammar`

Parsea una gramática en texto plano y detecta si tiene recursión izquierda.

**Request body:**
```json
{
  "raw_grammar": "S -> A B | a\nA -> a"
}
```

**Response exitosa:**
```json
{
  "status": "success",
  "has_left_recursion": false,
  "start_symbol": { "type": "NonTerminal", "value": "S" },
  "production_count": 3,
  "productions": [...]
}
```

**Response con error de parsing:**
```json
{
  "status": "error",
  "message": "Left side must be a Non-Terminal: a"
}
```

---

### `POST /parse-ll1`

Construye la tabla LL(1) para la gramática dada y simula el parsing de una cadena de entrada, retornando un snapshot por cada paso.

**Request body:**
```json
{
  "raw_grammar": "E -> T E'\nE' -> + T E' | epsilon\nT -> F T'\nT' -> * F T' | epsilon\nF -> ( E ) | id",
  "input_string": "id + id * id"
}
```

**Response exitosa:**
```json
{
  "status": "success",
  "parsing_table": {
    "E": { "id": "T E'", "(": "T E'" },
    "E'": { "+": "+ T E'", "$": "ϵ" }
  },
  "snapshots": [
    {
      "step": 0,
      "stack": ["$", "E"],
      "input_remaining": ["id", "+", "id", "*", "id", "$"],
      "action": "E -> T E'"
    }
  ]
}
```

**Response con error de gramática o conflicto LL(1):**
```json
{
  "status": "error",
  "message": "Grammar is not LL(1): Multiple entries for [A, a]"
}
```

> **Nota:** Los errores de parsing de cadena retornan `status: "error"` con HTTP 200, no 4xx, para que el frontend pueda mostrar el mensaje al usuario.

## Formato de gramática aceptado

```
S -> A B | a
A -> a A | epsilon
B -> b
```

| Regla | Ejemplo |
|---|---|
| Separador de producciones | `->` o `→` |
| Alternativas | `\|` |
| No-terminal | Primera letra **mayúscula** (`S`, `Expr`, `Term`) |
| Terminal | Primera letra **minúscula** o símbolo (`a`, `+`, `id`) |
| Épsilon | `ϵ`, `ε` o la palabra `epsilon` |
| Símbolo de fin | `$` (agregado automáticamente por el parser) |
| Símbolo inicial | La primera producción define el símbolo inicial |

## Tipos de datos principales

### `Symbol`
```rust
pub enum Symbol {
    Terminal(String),    // e.g. "a", "+", "id"
    NonTerminal(String), // e.g. "S", "E'"
    Epsilon,
}
```
Serializado en JSON con `{ "type": "Terminal", "value": "a" }`.

### `Production`
```rust
pub struct Production {
    pub left: Symbol,       // No-terminal del lado izquierdo
    pub right: Vec<Symbol>, // Cuerpo de la producción
}
```

### `ParseSnapshot`
Un snapshot representa el estado de la pila del parser en un paso de la simulación:
```rust
pub struct ParseSnapshot {
    pub step: usize,
    pub stack: Vec<Symbol>,
    pub input_remaining: Vec<String>,
    pub action: String, // Descripción legible de la acción tomada
}
```

## Implementación de los parsers

### LL(1) — `parsers/ll1.rs`

Implementación completa con tres fases:

1. **FIRST:** Calcula el conjunto FIRST de cada símbolo iterativamente hasta convergencia. Maneja épsilon propagado en secuencias.
2. **FOLLOW:** Calcula el conjunto FOLLOW de cada no-terminal usando el símbolo `$` como ancla del símbolo inicial. Propaga correctamente a través de épsilon.
3. **Tabla de parsing:** Para cada producción `A -> α`, agrega `α` en `M[A, a]` para cada `a ∈ FIRST(α)`. Si `ε ∈ FIRST(α)`, agrega para cada `b ∈ FOLLOW(A)`. Detecta conflictos y retorna error si la gramática no es LL(1).

**Simulación:** Stack-based. Toma `($, S)` como estado inicial. En cada paso: si el tope es un terminal, hace match con la entrada; si es un no-terminal, consulta la tabla y expande en orden inverso (para que el primer símbolo quede en el tope).

### LR(0) — `parsers/lr0.rs`

Estructura de datos implementada:
- `LR0Item`: Un ítem LR(0) con posición del punto (dot) dentro de una producción.
- `closure()`: Calcula la clausura de un conjunto de ítems expandiendo no-terminales después del punto.

La construcción completa de estados y tablas ACTION/GOTO está en desarrollo.

### Descenso recursivo — `parsers/recursive_descent.rs`

Placeholder. Implementado el trait `Parser` pero retorna error. Planificado para Fase 4.

## CORS

Configurado con `tower-http` para aceptar cualquier origen, método y cabecera. Permite consumo desde el frontend en `http://localhost:5173` sin restricciones durante desarrollo.

## Tokenización de la cadena de entrada

El endpoint `/parse-ll1` aplica la siguiente heurística sobre `input_string`:

- Si contiene espacios → split por espacios (`"id + id"` → `["id", "+", "id"]`)
- Si no contiene espacios → split carácter a carácter (`"abc"` → `["a", "b", "c"]`)
