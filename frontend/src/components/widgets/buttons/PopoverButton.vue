<template>
	<LayoutRow class="popover-button">
		<IconButton :action="() => onClick()" :icon="icon" :size="16" data-hover-menu-spawner />
		<FloatingMenu v-model:open="open" :type="'Popover'" :direction="'Bottom'">
			<slot></slot>
		</FloatingMenu>
	</LayoutRow>
</template>

<style lang="scss">
.popover-button {
	position: relative;
	width: 16px;
	height: 24px;
	flex: 0 0 auto;

	.floating-menu {
		left: 50%;
		bottom: 0;
	}

	.icon-button {
		width: 100%;
		height: 100%;
		padding: 0;
		outline: none;
		border: none;
		border-radius: 2px;
		background: var(--color-1-nearblack);
		fill: var(--color-e-nearwhite);

		&:hover {
			background: var(--color-6-lowergray);
			fill: var(--color-f-white);
		}
	}

	// TODO: Refactor this and other complicated cases dealing with joined widget margins and border-radius by adding a single standard set of classes: joined-first, joined-inner, and joined-last
	div[class*="-input"] + & {
		margin-left: 1px;

		.icon-button {
			border-radius: 0 2px 2px 0;
		}
	}
}
</style>

<script lang="ts">
import { defineComponent, PropType } from "vue";

import { IconName } from "@/utility-functions/icons";

import FloatingMenu from "@/components/floating-menus/FloatingMenu.vue";
import LayoutRow from "@/components/layout/LayoutRow.vue";
import IconButton from "@/components/widgets/buttons/IconButton.vue";

export default defineComponent({
	components: {
		FloatingMenu,
		IconButton,
		LayoutRow,
	},
	props: {
		icon: { type: String as PropType<IconName>, default: "DropdownArrow" },

		// Callbacks
		action: { type: Function as PropType<() => void>, required: false },
	},
	data() {
		return {
			open: false,
		};
	},
	methods: {
		onClick() {
			this.open = true;

			this.action?.();
		},
	},
});
</script>
