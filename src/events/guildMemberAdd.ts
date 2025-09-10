import {
    type GuildMember,
    EmbedBuilder,
    ActionRowBuilder,
    ButtonBuilder,
    ButtonStyle,
    Events
} from 'discord.js';
import { Event } from '../types/Event.js';

export const event: Event = {
    name: Events.GuildMemberAdd,
    once: false,

    async execute(member: GuildMember) {
        const welcomeChannelId = process.env.WELCOME_CHANNEL_ID!;
        const channel = member.guild.channels.cache.get(welcomeChannelId);
        if (!channel?.isTextBased()) {
            return;
        }

        const welcomeEmbed = new EmbedBuilder()
            .setTitle(`👋 Welcome ${member.user.username}!`)
            .setDescription('Click on the green button below to join Le Socle.')
            .setColor(0x00bfff);

        const button = new ButtonBuilder()
            .setCustomId(`welcome-role-${member.id}`)
            .setLabel('Join Le Socle.')
            .setStyle(ButtonStyle.Success);

        const row = new ActionRowBuilder<ButtonBuilder>().addComponents(button);

        await channel.send({
            content: `<@${member.id}>`,
            embeds: [welcomeEmbed],
            components: [row],
        });
    },
};
