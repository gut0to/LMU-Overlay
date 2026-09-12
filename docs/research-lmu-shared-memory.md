# Pesquisa: interface oficial de Shared Memory LMU/rFactor para HashOverlay

Data da pesquisa: 2026-09-12.

## Escopo e regra de fonte

Esta pesquisa usou somente fontes primarias ou oficiais: paginas e downloads oficiais da Studio 397/rFactor, pacote oficial de exemplo de plugin rFactor 2, pacote oficial de Rules Plugin, nota oficial de release da LMU via Steam News API, e documentacao oficial da LMU/Studio 397. Nao foram usados offsets ou layouts de projetos comunitarios como fonte de verdade.

Fontes principais:

- Studio 397, [Modding Resources](https://www.studio-397.com/modding-resources/): lista oficial do `rFactor2 internals plugin`, incluindo `Instruction`, `Example Plugin v8` e `Rules Plugin Example Source Code`.
- Studio 397, [rFactorInternalsPlugin.pdf](https://www.studio-397.com/wp-content/uploads/2016/12/rFactorInternalsPlugin.pdf): descreve que a arquitetura de Internals Plugin expoe dados internos para desenvolvedores, incluindo telemetria, motion bases, head trackers e bancos estatisticos; tambem diz que os membros da classe de telemetria estao expostos e comentados nos fontes inclusos.
- Studio 397, [`rF2_Example_Plugin.7z`](https://www.studio-397.com/wp-content/uploads/2023/03/rF2_Example_Plugin.7z): pacote oficial baixado da pagina de Modding Resources; contem `Include/InternalsPlugin.hpp`.
- Studio 397, [`RulesPluginExampleSourceCodev1.zip`](https://www.studio-397.com/wp-content/uploads/2016/12/RulesPluginExampleSourceCodev1.zip): pacote oficial baixado da pagina de Modding Resources; contem exemplos oficiais de regras e outra copia de `InternalsPlugin.hpp`.
- rFactor.net, [pagina oficial rFactor](https://www.rfactor.net/) e [downloads oficiais](https://rfactor.net/downloads/): confirmam o sistema oficial de Internals Plugin para extrair standings/telemetria em rFactor.
- Steam News API / anuncio oficial LMU, [V1.2 Update - patch notes](https://steamstore-a.akamaihd.net/news/externalpost/steam_community_announcements/1818118366184999): em `Engine & Tech`, a LMU V1.2 adicionou um `SharedMemory Header` na pasta `Support` para a nova implementacao de shared memory.
- Le Mans Ultimate Guide, [Known Issues and Advice](https://guide.lemansultimate.com/hc/en-gb): confirma caminhos oficiais de plugins/configuracao da LMU, como `Le Mans Ultimate\UserData\player\CustomPluginVariables.JSON`, mas nao publica o layout de shared memory.

Observacao: a maquina atual nao tem LMU instalado no Steam local; portanto nao foi possivel abrir a copia oficial do header em `Le Mans Ultimate\Support\SharedMemoryInterface`. A conclusao abaixo evita afirmar offsets de `LMU_Data` que nao tenham sido confirmados pelo header oficial da LMU.

## O que a fonte oficial LMU confirma

A nota oficial da LMU V1.2 confirma a existencia de uma nova implementacao de shared memory e que o header foi adicionado ao diretorio `Support`. Isso torna o header distribuido com o jogo a fonte oficial para o layout de `LMU_Data`.

Limite: a nota de release nao publica nomes de structs, campos, tamanhos, offsets ou versoes do layout. Sem a copia local do header da LMU, nao ha base primaria suficiente para declarar offsets novos em HashOverlay.

## Campos oficiais no rFactor 2 Internals Plugin v8

O pacote oficial `rF2_Example_Plugin.7z`, em `Include/InternalsPlugin.hpp`, define `TelemInfoV01`, `VehicleScoringInfoV01`, `ScoringInfoV01` e `TrackRulesV01`. Estes nomes sao oficiais para a API de plugin rFactor 2, mas ainda precisam ser reconciliados com o header LMU `SharedMemoryInterface` antes de virar offset em `LMU_Data`.

### Volta, setor e validade/contagem

Campos oficiais encontrados:

- `TelemInfoV01::mLapNumber`: numero da volta atual.
- `TelemInfoV01::mLapStartET`: tempo de inicio da volta atual.
- `TelemInfoV01::mCurrentSector`: setor atual zero-based, com pitlane armazenado no bit de sinal.
- `VehicleScoringInfoV01::mTotalLaps`: voltas completadas.
- `VehicleScoringInfoV01::mSector`: `0=sector3`, `1=sector1`, `2=sector2`.
- `VehicleScoringInfoV01::mLapDist`: distancia atual ao redor da pista.
- `VehicleScoringInfoV01::mCountLapFlag`: `0 = do not count lap or time`, `1 = count lap but not time`, `2 = count lap and time`.
- `VehicleScoringInfoV01::mServerScored`: se o veiculo esta sendo pontuado pelo servidor.
- `VehicleScoringInfoV01::mInPits`, `mPitState`, `mInGarageStall`: estado de pits/box.

Conclusao de validade: `mCountLapFlag` e o melhor candidato oficial para distinguir volta/tempo contado versus nao contado. Ele nao e documentado como "track cut" ou "lap invalidated" especificamente; e um campo de contagem/pontuacao. Para HashOverlay, ele pode alimentar `lap_invalidated` somente depois de confirmar que o header LMU expoe o mesmo campo em `LMU_Data`.

### Penalidades e track limits

Campos oficiais encontrados:

- `VehicleScoringInfoV01::mNumPenalties`: numero de penalidades pendentes.
- `VehicleScoringInfoV01::mFinishStatus`: inclui `dq`.
- `VehicleScoringInfoV01::mFlag`: bandeira primaria mostrada ao veiculo, documentada no header como atualmente `0=green` ou `6=blue`.
- `ScoringInfoV01::mYellowFlagState` e `mSectorFlag[3]`: estado de bandeira amarela full-course/local por setor.
- `TrackRulesV01` e `TrackRulesParticipantV01`: API oficial para regras de formacao/caution, incluindo mensagens, pits open, safety car, yellow severity e ordenacao.

Nao encontrei, nas fontes oficiais lidas, um campo de scoring chamado "track limits incident points", "last cut", "lap invalidated by track limits" ou equivalente direto. A nota oficial da LMU V1.2 menciona mudancas de gameplay para track limits e time penalties, mas nao diz que esses dados foram expostos em shared memory.

Conclusao de penalidades: `mNumPenalties` e mapeavel como "penalidades pendentes" se existir no header LMU. Track limits detalhado e motivo de invalidacao ficam limitacao ate o header LMU oficial expor campo especifico.

### Tempos de setores e melhores tempos

Campos oficiais encontrados em `VehicleScoringInfoV01`:

- `mBestSector1`: melhor S1.
- `mBestSector2`: melhor S2 cumulativo, descrito no header como "plus sector 1".
- `mBestLapTime`: melhor volta.
- `mLastSector1`: ultimo S1.
- `mLastSector2`: ultimo S2 cumulativo, descrito como "plus sector 1".
- `mLastLapTime`: ultima volta.
- `mCurSector1`: S1 atual se valido.
- `mCurSector2`: S2 atual cumulativo se valido.
- `mBestLapSector1`: S1 da melhor volta, nao necessariamente o melhor S1 absoluto.
- `mBestLapSector2`: S2 da melhor volta, nao necessariamente o melhor S2 absoluto.
- Nao ha `mCurSector3` nem `mLastSector3`: o proprio header comenta que nao ha current lap time porque ele vira "last" instantaneamente.

Implicacao pratica:

- S1 real: usar `mCurSector1` durante/apos S1 quando valido.
- S2 real isolado: calcular `mCurSector2 - mCurSector1`, porque `mCurSector2` e cumulativo.
- Ultimo S3 isolado: calcular `mLastLapTime - mLastSector2`, porque `mLastSector2` e cumulativo.
- Melhor S2 isolado: `mBestSector2 - mBestSector1`, assumindo que ambos pertencem ao mesmo melhor parcial cumulativo conforme o contrato rFactor.
- Melhor S3 absoluto nao esta exposto diretamente. Pode-se calcular S3 da melhor volta como `mBestLapTime - mBestLapSector2`, mas isso e setor 3 da melhor volta, nao necessariamente o melhor S3 absoluto.

### Track layout

Campos oficiais relacionados a pista:

- `TelemInfoV01::mTrackName` e `ScoringInfoV01::mTrackName`: nome da pista atual.
- `ScoringInfoV01::mLapDist`: comprimento/distancia da volta.
- `VehicleScoringInfoV01::mLapDist`: posicao longitudinal do veiculo na volta.
- `VehicleScoringInfoV01::mPathLateral`: posicao lateral relativa a um caminho central aproximado.
- `VehicleScoringInfoV01::mTrackEdge`: borda da pista no mesmo lado do veiculo, relativa ao caminho central aproximado.
- `TrackRulesV01::mPitLaneStartDist`, `mTeleportLapDist`, `mSafetyCarLapDist`: distancias pontuais usadas por regras.

Nao ha, no header oficial rFactor 2 v8, um mapa geometrico de track layout, polilinha da pista, lista de curvas, setores por coordenada, AIW completo ou spline. Para layout visual, HashOverlay precisa de outra fonte oficial publicada, arquivo de conteudo permitido, ou derivacao propria por telemetria gravada; nao deve inventar offsets.

## Conclusao pratica para HashOverlay

Mapeavel agora no leitor existente, porque o repo ja possui campos de `LMU_Data` implementados e alinhados ao escopo atual:

- nome da pista;
- nome/classe do veiculo;
- sessao/fase;
- volta atual;
- inicio da volta;
- setor atual;
- distancia na volta;
- comprimento da pista;
- estado de pits/garagem;
- controles e telemetria basica ja existentes.

Mapeavel como proximo passo somente apos abrir o header oficial LMU em `Support\SharedMemoryInterface` e confirmar nomes/layout:

- `mCountLapFlag` para contagem/validade de volta/tempo;
- `mNumPenalties` para penalidades pendentes;
- `mBestSector1`, `mBestSector2`, `mBestLapTime`;
- `mLastSector1`, `mLastSector2`, `mLastLapTime`;
- `mCurSector1`, `mCurSector2`;
- `mBestLapSector1`, `mBestLapSector2`;
- `mServerScored`, `mPitState`, `mFlag`, se uteis para a regra de validade conservadora.

Limitacao ate haver fonte oficial adicional:

- invalidacao especifica por track limits/cut;
- pontos de track limits/incidentes;
- motivo textual ou tipo de penalidade;
- melhor setor 3 absoluto;
- tempo real de S3 antes da volta fechar;
- track layout geometrico pronto para desenhar mapa.

Recomendacao: nao adicionar nenhum offset novo ao HashOverlay ate que uma copia oficial do header LMU atual seja lida e comparada contra o buffer `LMU_Data`. O campo `mCountLapFlag` e o candidato mais forte para lap validity/count-lap, mas, sem o header LMU local, ele deve permanecer documentado como candidato oficial rFactor 2, nao como offset confirmado de LMU.
