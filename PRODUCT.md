# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Pilotos de Le Mans Ultimate que precisam configurar e operar um overlay de telemetria antes e durante sessões de corrida. Esta descrição é uma inferência baseada no código e no pedido de redesign.

## Product Purpose

HashOverlay lê a telemetria do LMU em modo somente leitura e apresenta dados de condução em superfícies configuráveis. O Settings permite iniciar e parar o host, configurar as superfícies e salvar preferências do overlay.

## Positioning

Um painel local de operação que conecta uma configuração detalhada de telemetria a um runtime multi-surface, sem depender de serviços remotos.

## Operating Context

O produto é usado em desktop Windows, normalmente entre treinos, qualificação e corrida. O operador alterna rapidamente entre estado do host, widgets, posicionamento, aparência, timing, coaching e atalhos.

## Capabilities and Constraints

- Aplicação React/Vite distribuída dentro de um wrapper Tauri.
- O backend e o formato de configuração existentes devem permanecer compatíveis.
- O host usa IPC local e shared memory do LMU em modo somente leitura.
- O redesign solicitado substitui a interface visual, não a funcionalidade do runtime.

## Brand Commitments

Nome confirmado: HashOverlay. O pedido exige uma paleta integralmente nova e uma interface sem estética genérica de IA.

## Evidence on Hand

- Implementação atual: `settings/src/App.tsx` e `settings/src/styles.css`.
- Catálogo funcional de widgets e tipos de configuração no repositório.
- Não há ativos de marca, fotografias ou conteúdo externo aprovados para uso.

## Product Principles

- Estado operacional deve ser legível em segundos.
- Alterações de configuração devem permanecer explícitas e reversíveis.
- Densidade de controle não pode sacrificar orientação.
- O visual deve servir à corrida e não competir com ela.
