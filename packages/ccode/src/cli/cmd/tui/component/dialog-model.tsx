import { createMemo, createSignal } from "solid-js"
import { useLocal } from "@tui/context/local"
import { useSync } from "@tui/context/sync"
import { DialogSelect, type DialogSelectRef } from "@tui/ui/dialog-select"
import { useDialog } from "@tui/ui/dialog"
import { createDialogProviderOptions, DialogProvider } from "./dialog-provider"
import { useKeybind } from "../context/keybind"
import * as fuzzysort from "fuzzysort"

export function useConnected() {
  const sync = useSync()
  return createMemo(() =>
    sync.data.provider.some(
      (x) => x.id !== "ccode" || Object.values(x.models).some((y: any) => y.cost?.input !== 0),
    ),
  )
}

export function DialogModel(props: { providerID?: string }) {
  const local = useLocal()
  const sync = useSync()
  const dialog = useDialog()
  const keybind = useKeybind()
  const [ref, setRef] = createSignal<DialogSelectRef<unknown>>()
  const [query, setQuery] = createSignal("")

  const connected = useConnected()
  const providers = createDialogProviderOptions()

  const showExtra = createMemo(() => {
    if (!connected()) return false
    if (props.providerID) return false
    return true
  })

  const options = createMemo(() => {
    const q = query()
    const needle = q.trim()
    const showSections = showExtra() && needle.length === 0
    const favorites = connected() ? local.model.favorite() : []
    const recents = local.model.recent()

    const recentList = showSections
      ? recents.filter(
          (item) => !favorites.some((fav) => fav.providerID === item.providerID && fav.modelID === item.modelID),
        )
      : []

    const favoriteOptions = showSections
      ? favorites.flatMap((item) => {
          const provider = sync.data.provider.find((x) => x.id === item.providerID)
          if (!provider) return []
          const model = provider.models[item.modelID]
          if (!model) return []
          return [
            {
              key: item,
              value: {
                providerID: provider.id,
                modelID: model.id,
              },
              title: model.name ?? item.modelID,
              description: provider.name,
              category: "Favorites",
              disabled: provider.id === "ccode" && model.id.includes("-nano"),
              footer: model.cost?.input === 0 && provider.id === "ccode" ? "Free" : undefined,
              onSelect: () => {
                dialog.clear()
                local.model.set(
                  {
                    providerID: provider.id,
                    modelID: model.id,
                  },
                  { recent: true },
                )
              },
            },
          ]
        })
      : []

    const recentOptions = showSections
      ? recentList.flatMap((item) => {
          const provider = sync.data.provider.find((x) => x.id === item.providerID)
          if (!provider) return []
          const model = provider.models[item.modelID]
          if (!model) return []
          return [
            {
              key: item,
              value: {
                providerID: provider.id,
                modelID: model.id,
              },
              title: model.name ?? item.modelID,
              description: provider.name,
              category: "Recent",
              disabled: provider.id === "ccode" && model.id.includes("-nano"),
              footer: model.cost?.input === 0 && provider.id === "ccode" ? "Free" : undefined,
              onSelect: () => {
                dialog.clear()
                local.model.set(
                  {
                    providerID: provider.id,
                    modelID: model.id,
                  },
                  { recent: true },
                )
              },
            },
          ]
        })
      : []

    // Build provider options using plain JavaScript
    const providerList = sync.data.provider
    if (!providerList || providerList.length === 0) return [...favoriteOptions, ...recentOptions]
    const sortedProviders = Array.from(providerList).sort((a, b) => {
      if ((a.id !== "ccode") !== (b.id !== "ccode")) return a.id !== "ccode" ? 1 : -1
      return a.name.localeCompare(b.name)
    })

    const providerOptions = sortedProviders.flatMap((provider) => {
      const models = provider.models
      if (!models || typeof models !== "object") return []
      return (Object.entries(models) as [string, any][])
        .filter(([_, info]) => info.status !== "deprecated")
        .filter(([_, info]) => (props.providerID ? info.providerID === props.providerID : true))
        .map(([model, info]) => {
          const value = {
            providerID: provider.id,
            modelID: model,
          }
          return {
            value,
            title: info.name ?? model,
            description: favorites.some(
              (item) => item.providerID === value.providerID && item.modelID === value.modelID,
            )
              ? "(Favorite)"
              : undefined,
            category: connected() ? provider.name : undefined,
            disabled: provider.id === "ccode" && model.includes("-nano"),
            footer: info.cost?.input === 0 && provider.id === "ccode" ? "Free" : undefined,
            onSelect() {
              dialog.clear()
              local.model.set(
                {
                  providerID: provider.id,
                  modelID: model,
                },
                { recent: true },
              )
            },
          }
        })
        .filter((x) => {
          if (!showSections) return true
          const value = x.value
          const inFavorites = favorites.some(
            (item) => item.providerID === value.providerID && item.modelID === value.modelID,
          )
          if (inFavorites) return false
          const inRecents = recentList.some(
            (item) => item.providerID === value.providerID && item.modelID === value.modelID,
          )
          if (inRecents) return false
          return true
        })
        .sort((a, b) => {
          if ((a.footer !== "Free") !== (b.footer !== "Free")) return a.footer !== "Free" ? 1 : -1
          return a.title.localeCompare(b.title)
        })
    })

    const providersList = providers()
    const popularProviders = !connected() && providersList && providersList.length > 0
      ? Array.from(providersList)
          .map((option) => ({
            ...option,
            category: "Popular providers",
          }))
          .slice(0, 6)
      : []

    // Search shows a single merged list (favorites inline)
    if (needle) {
      const filteredProviders = fuzzysort.go(needle, providerOptions, { keys: ["title", "category"] }).map((x) => x.obj)
      const filteredPopular = fuzzysort.go(needle, popularProviders, { keys: ["title"] }).map((x) => x.obj)
      return [...filteredProviders, ...filteredPopular]
    }

    return [...favoriteOptions, ...recentOptions, ...providerOptions, ...popularProviders]
  })

  const provider = createMemo(() =>
    props.providerID ? sync.data.provider.find((x) => x.id === props.providerID) : null,
  )

  const title = createMemo(() => {
    if (provider()) return provider()!.name
    return "Select model"
  })

  return (
    <DialogSelect
      keybind={[
        {
          keybind: keybind.all.model_provider_list?.[0],
          title: connected() ? "Connect provider" : "View all providers",
          onTrigger() {
            dialog.replace(() => <DialogProvider />)
          },
        },
        {
          keybind: keybind.all.model_favorite_toggle?.[0],
          title: "Favorite",
          disabled: !connected(),
          onTrigger: (option) => {
            local.model.toggleFavorite(option.value as { providerID: string; modelID: string })
          },
        },
      ]}
      ref={setRef}
      onFilter={setQuery}
      skipFilter={true}
      title={title()}
      current={local.model.current()}
      options={options()}
    />
  )
}
